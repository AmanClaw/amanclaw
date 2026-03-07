use amanclaw_plugin_sdk::*;
use std::collections::HashSet;
use std::process::Command;

const ALLOWED_COMMANDS: &[&str] = &[
    "ls", "cat", "grep", "find", "df", "free", "uptime", "date", "wc",
    "head", "tail", "sort", "uniq", "du", "whoami", "hostname", "pwd",
];

pub fn metadata() -> SkillMetadata {
    SkillMetadata {
        name: "run_command".into(),
        description: "Run a safe, whitelisted shell command".into(),
        timeout_ms: 30000,
        version: "0.1.0".into(),
    }
}

pub fn parameters() -> serde_json::Value {
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

pub fn execute(input: SkillInput) -> SkillResult {
    let args: serde_json::Value = match serde_json::from_str(&input.args) {
        Ok(v) => v,
        Err(e) => return SkillResult::err(format!("Invalid args: {}", e)),
    };

    let command = match args.get("command").and_then(|v| v.as_str()) {
        Some(c) => c,
        None => return SkillResult::err("Missing required parameter: command"),
    };

    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return SkillResult::err("Empty command");
    }

    let cmd_name = parts[0];
    let allowed: HashSet<&str> = ALLOWED_COMMANDS.iter().copied().collect();

    if !allowed.contains(cmd_name) {
        return SkillResult::err(format!(
            "Command '{}' not allowed. Allowed: {}",
            cmd_name,
            ALLOWED_COMMANDS.join(", ")
        ));
    }

    // Reject dangerous patterns
    if command.contains('|') || command.contains(';') || command.contains('&')
        || command.contains('`') || command.contains("$(")
    {
        return SkillResult::err("Pipes, chains, and subshells are not allowed");
    }

    match Command::new(cmd_name)
        .args(&parts[1..])
        .output()
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            if output.status.success() {
                let result = if stdout.len() > 2000 {
                    format!("{}...\n(truncated)", &stdout[..2000])
                } else {
                    stdout.to_string()
                };
                SkillResult::ok(result)
            } else {
                SkillResult::err(format!("Command failed: {}", stderr))
            }
        }
        Err(e) => SkillResult::err(format!("Failed to execute: {}", e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_input(command: &str) -> SkillInput {
        SkillInput {
            name: "run_command".into(),
            args: serde_json::json!({"command": command}).to_string(),
            user_id: "test".into(),
            platform: "test".into(),
        }
    }

    #[test]
    fn test_allowed_command() {
        let result = execute(make_input("whoami"));
        assert!(result.success);
    }

    #[test]
    fn test_blocked_command() {
        let result = execute(make_input("rm -rf /"));
        assert!(!result.success);
        assert!(result.error.unwrap().contains("not allowed"));
    }

    #[test]
    fn test_pipe_rejected() {
        let result = execute(make_input("ls | grep foo"));
        assert!(!result.success);
        assert!(result.error.unwrap().contains("not allowed"));
    }

    #[test]
    fn test_subshell_rejected() {
        let result = execute(make_input("ls $(whoami)"));
        assert!(!result.success);
    }
}
