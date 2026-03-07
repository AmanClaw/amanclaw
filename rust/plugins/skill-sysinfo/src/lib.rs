use amanclaw_plugin_sdk::*;
use sysinfo::System;

pub fn metadata() -> SkillMetadata {
    SkillMetadata {
        name: "system_info".into(),
        description: "Get current CPU, memory, and disk usage".into(),
        timeout_ms: 5000,
        version: "0.1.0".into(),
    }
}

pub fn parameters() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {},
        "required": []
    })
}

pub fn execute(_input: SkillInput) -> SkillResult {
    let mut sys = System::new_all();
    sys.refresh_all();

    let total_mem = sys.total_memory() / 1024 / 1024;
    let used_mem = sys.used_memory() / 1024 / 1024;
    let cpu_usage = sys.global_cpu_usage();

    let output = format!(
        "CPU: {:.1}%\nMemory: {} MB / {} MB ({:.1}%)\nProcesses: {}",
        cpu_usage,
        used_mem, total_mem,
        (used_mem as f64 / total_mem as f64) * 100.0,
        sys.processes().len(),
    );

    SkillResult::ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata() {
        let meta = metadata();
        assert_eq!(meta.name, "system_info");
    }

    #[test]
    fn test_execute_returns_info() {
        let input = SkillInput {
            name: "system_info".into(),
            args: "{}".into(),
            user_id: "test".into(),
            platform: "test".into(),
        };
        let result = execute(input);
        assert!(result.success);
        assert!(result.output.contains("CPU:"));
        assert!(result.output.contains("Memory:"));
    }
}
