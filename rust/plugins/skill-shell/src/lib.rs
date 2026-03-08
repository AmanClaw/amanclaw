use amanclaw_traits::skill::{Skill, SkillMetadata, SkillInput, SkillResult};
use std::collections::HashSet;
use std::process::Command;

const ALLOWED_COMMANDS: &[&str] = &[
    "ls", "cat", "grep", "find", "df", "free", "uptime", "date", "wc",
    "head", "tail", "sort", "uniq", "du", "whoami", "hostname", "pwd",
];

pub struct ShellSkill;

#[async_trait::async_trait]
impl Skill for ShellSkill {
    fn metadata(&self) -> SkillMetadata {
        SkillMetadata {
            name: "run_command".into(),
            description: "Run a safe, whitelisted shell command (ls, cat, grep, find, df, free, uptime, date, etc.)".into(),
            timeout_ms: 30000,
            version: "0.1.0".into(),
        }
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to run (must be whitelisted)"
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, input: SkillInput) -> SkillResult {
        execute_shell(&input.args)
    }
}

fn execute_shell(args_str: &str) -> SkillResult {
    let args: serde_json::Value = match serde_json::from_str(args_str) {
        Ok(v) => v,
        Err(e) => return SkillResult { success: false, output: String::new(), error: Some(format!("Invalid args: {}", e)) },
    };

    let command = match args.get("command").and_then(|v| v.as_str()) {
        Some(c) => c,
        None => return SkillResult { success: false, output: String::new(), error: Some("Missing required parameter: command".into()) },
    };

    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return SkillResult { success: false, output: String::new(), error: Some("Empty command".into()) };
    }

    let cmd_name = parts[0];
    let allowed: HashSet<&str> = ALLOWED_COMMANDS.iter().copied().collect();

    if !allowed.contains(cmd_name) {
        return SkillResult {
            success: false, output: String::new(),
            error: Some(format!("Command '{}' not allowed. Allowed: {}", cmd_name, ALLOWED_COMMANDS.join(", "))),
        };
    }

    if command.contains('|') || command.contains(';') || command.contains('&')
        || command.contains('`') || command.contains("$(")
    {
        return SkillResult { success: false, output: String::new(), error: Some("Pipes, chains, and subshells are not allowed".into()) };
    }

    match Command::new(cmd_name).args(&parts[1..]).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            if output.status.success() {
                let result = if stdout.len() > 2000 {
                    format!("{}...\n(truncated)", &stdout[..2000])
                } else {
                    stdout.to_string()
                };
                SkillResult { success: true, output: result, error: None }
            } else {
                SkillResult { success: false, output: String::new(), error: Some(format!("Command failed: {}", stderr)) }
            }
        }
        Err(e) => SkillResult { success: false, output: String::new(), error: Some(format!("Failed to execute: {}", e)) },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_args(command: &str) -> String {
        serde_json::json!({"command": command}).to_string()
    }

    #[test]
    fn test_allowed_command() {
        let result = execute_shell(&make_args("whoami"));
        assert!(result.success);
    }

    #[test]
    fn test_blocked_command() {
        let result = execute_shell(&make_args("rm -rf /"));
        assert!(!result.success);
        assert!(result.error.unwrap().contains("not allowed"));
    }

    #[test]
    fn test_pipe_rejected() {
        let result = execute_shell(&make_args("ls | grep foo"));
        assert!(!result.success);
        assert!(result.error.unwrap().contains("not allowed"));
    }

    #[test]
    fn test_subshell_rejected() {
        let result = execute_shell(&make_args("ls $(whoami)"));
        assert!(!result.success);
    }
}
