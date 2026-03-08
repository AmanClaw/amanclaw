//! Script Plugin Runtime — loads plugins written in Python, JavaScript, or any language
//! that can communicate via JSON over stdin/stdout.
//!
//! Protocol:
//! - Engine sends a JSON line to the process stdin
//! - Plugin responds with a JSON line to stdout
//!
//! Commands:
//! - `{"method": "metadata"}` → returns SkillMetadata JSON
//! - `{"method": "parameters"}` → returns parameters JSON schema
//! - `{"method": "execute", "input": {...}}` → returns SkillResult JSON
//! - `{"method": "shutdown"}` → plugin exits

use amanclaw_traits::skill::{Skill, SkillInput, SkillMetadata, SkillResult};
use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

/// A skill backed by an external script process.
pub struct ScriptSkill {
    metadata: SkillMetadata,
    parameters_json: Value,
    command: String,
    args: Vec<String>,
    env: HashMap<String, String>,
    process: Mutex<Option<ScriptProcess>>,
}

struct ScriptProcess {
    child: Child,
    stdin: tokio::process::ChildStdin,
    stdout: BufReader<tokio::process::ChildStdout>,
}

impl ScriptSkill {
    /// Spawn the script process and query its metadata.
    pub async fn load(
        command: &str,
        args: &[String],
        env: &HashMap<String, String>,
    ) -> Result<Self> {
        let (proc, metadata, parameters) = Self::spawn_and_init(command, args, env).await?;

        tracing::info!(
            name = %metadata.name,
            command = %command,
            "Loaded script plugin"
        );

        Ok(Self {
            metadata,
            parameters_json: parameters,
            command: command.to_string(),
            args: args.to_vec(),
            env: env.clone(),
            process: Mutex::new(Some(proc)),
        })
    }

    async fn spawn_and_init(
        command: &str,
        args: &[String],
        env: &HashMap<String, String>,
    ) -> Result<(ScriptProcess, SkillMetadata, Value)> {
        let mut cmd = Command::new(command);
        cmd.args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        for (key, value) in env {
            cmd.env(key, value);
        }

        let mut child = cmd.spawn()
            .with_context(|| format!("Failed to spawn script plugin: {} {:?}", command, args))?;

        let stdin = child.stdin.take()
            .ok_or_else(|| anyhow::anyhow!("Failed to get stdin for script plugin"))?;
        let stdout = child.stdout.take()
            .ok_or_else(|| anyhow::anyhow!("Failed to get stdout for script plugin"))?;

        let mut proc = ScriptProcess {
            child,
            stdin,
            stdout: BufReader::new(stdout),
        };

        // Query metadata
        let meta_resp = Self::call_process(&mut proc, serde_json::json!({"method": "metadata"})).await?;
        let metadata: SkillMetadata = serde_json::from_value(meta_resp)
            .with_context(|| "Invalid metadata from script plugin")?;

        // Query parameters
        let params_resp = Self::call_process(&mut proc, serde_json::json!({"method": "parameters"})).await?;

        Ok((proc, metadata, params_resp))
    }

    async fn call_process(proc: &mut ScriptProcess, request: Value) -> Result<Value> {
        let mut json = serde_json::to_string(&request)?;
        json.push('\n');

        proc.stdin.write_all(json.as_bytes()).await?;
        proc.stdin.flush().await?;

        let mut line = String::new();
        proc.stdout.read_line(&mut line).await?;

        let resp: Value = serde_json::from_str(line.trim())
            .with_context(|| format!("Invalid JSON from script plugin: {}", line.trim()))?;

        Ok(resp)
    }

    /// Restart the process if it died.
    async fn ensure_running(&self) -> Result<()> {
        let mut guard = self.process.lock().await;
        if let Some(ref mut proc) = *guard {
            // Check if still running
            match proc.child.try_wait() {
                Ok(Some(_)) => {
                    // Process exited, respawn
                    tracing::warn!(name = %self.metadata.name, "Script plugin exited, respawning");
                    let (new_proc, _, _) = Self::spawn_and_init(
                        &self.command, &self.args, &self.env
                    ).await?;
                    *guard = Some(new_proc);
                }
                Ok(None) => {} // Still running
                Err(_) => {}   // Can't check, assume running
            }
        } else {
            let (new_proc, _, _) = Self::spawn_and_init(
                &self.command, &self.args, &self.env
            ).await?;
            *guard = Some(new_proc);
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl Skill for ScriptSkill {
    fn metadata(&self) -> SkillMetadata {
        self.metadata.clone()
    }

    fn parameters_schema(&self) -> Value {
        self.parameters_json.clone()
    }

    async fn execute(&self, input: SkillInput) -> SkillResult {
        if let Err(e) = self.ensure_running().await {
            return SkillResult {
                success: false,
                output: String::new(),
                error: Some(format!("Failed to start script plugin: {}", e)),
            };
        }

        let mut guard = self.process.lock().await;
        let proc = match guard.as_mut() {
            Some(p) => p,
            None => return SkillResult {
                success: false,
                output: String::new(),
                error: Some("Script plugin not running".into()),
            },
        };

        let request = serde_json::json!({
            "method": "execute",
            "input": {
                "name": input.name,
                "args": input.args,
                "user_id": input.user_id,
                "platform": input.platform,
            }
        });

        match Self::call_process(proc, request).await {
            Ok(resp) => {
                serde_json::from_value(resp.clone()).unwrap_or_else(|_| SkillResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Invalid result from script plugin: {}", resp)),
                })
            }
            Err(e) => SkillResult {
                success: false,
                output: String::new(),
                error: Some(format!("Script plugin error: {}", e)),
            },
        }
    }
}

impl Drop for ScriptSkill {
    fn drop(&mut self) {
        // Send shutdown and kill process
        if let Ok(mut guard) = self.process.try_lock() {
            if let Some(ref mut proc) = *guard {
                let _ = proc.child.start_kill();
            }
        }
    }
}

/// Load all script plugins from a config map.
pub async fn load_script_plugins(
    configs: &HashMap<String, ScriptPluginConfig>,
) -> Vec<Arc<dyn Skill>> {
    let mut skills: Vec<Arc<dyn Skill>> = Vec::new();

    for (name, config) in configs {
        match ScriptSkill::load(&config.command, &config.args, &config.env).await {
            Ok(skill) => {
                tracing::info!(
                    name = %name,
                    skill_name = %skill.metadata.name,
                    "Script plugin loaded"
                );
                skills.push(Arc::new(skill));
            }
            Err(e) => {
                tracing::error!(
                    name = %name,
                    error = %e,
                    "Failed to load script plugin"
                );
            }
        }
    }

    skills
}

/// Configuration for a script plugin.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScriptPluginConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

/// Discover script plugins from a directory.
/// Looks for `*.py` files and wraps them with `python3`.
pub async fn discover_script_plugins(dir: &Path) -> Vec<Arc<dyn Skill>> {
    let mut skills: Vec<Arc<dyn Skill>> = Vec::new();

    if !dir.exists() {
        return skills;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            tracing::error!(error = %e, "Failed to read script plugins directory");
            return skills;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        let (command, args) = match ext {
            "py" => ("python3".to_string(), vec![path.to_string_lossy().to_string()]),
            "js" | "mjs" => ("node".to_string(), vec![path.to_string_lossy().to_string()]),
            _ => continue,
        };

        match ScriptSkill::load(&command, &args, &HashMap::new()).await {
            Ok(skill) => {
                tracing::info!(
                    name = %skill.metadata.name,
                    path = %path.display(),
                    "Discovered script plugin"
                );
                skills.push(Arc::new(skill));
            }
            Err(e) => {
                tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    "Failed to load script plugin"
                );
            }
        }
    }

    skills
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_script_plugin_config() {
        let yaml = r#"
command: "python3"
args: ["plugin.py"]
env:
  API_KEY: "test"
"#;
        let config: ScriptPluginConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.command, "python3");
        assert_eq!(config.args, vec!["plugin.py"]);
        assert_eq!(config.env["API_KEY"], "test");
    }

    #[test]
    fn test_discover_nonexistent_dir() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let skills = rt.block_on(discover_script_plugins(Path::new("/tmp/nonexistent-scripts")));
        assert!(skills.is_empty());
    }
}
