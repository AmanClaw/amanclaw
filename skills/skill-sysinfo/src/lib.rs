use amanclaw_traits::skill::{Skill, SkillInput, SkillMetadata, SkillResult};
use sysinfo::System;

pub struct SysInfoSkill;

#[async_trait::async_trait]
impl Skill for SysInfoSkill {
    fn metadata(&self) -> SkillMetadata {
        SkillMetadata {
            name: "system_info".into(),
            description: "Get current CPU, memory, and disk usage of the host system".into(),
            timeout_ms: 5000,
            version: "0.1.0".into(),
        }
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn execute(&self, _input: SkillInput) -> SkillResult {
        let mut sys = System::new_all();
        sys.refresh_all();

        let total_mem = sys.total_memory() / 1024 / 1024;
        let used_mem = sys.used_memory() / 1024 / 1024;
        let cpu_usage = sys.global_cpu_usage();

        let output = format!(
            "CPU: {:.1}%\nMemory: {} MB / {} MB ({:.1}%)\nProcesses: {}",
            cpu_usage,
            used_mem,
            total_mem,
            (used_mem as f64 / total_mem as f64) * 100.0,
            sys.processes().len(),
        );

        SkillResult {
            success: true,
            output,
            error: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata() {
        let skill = SysInfoSkill;
        assert_eq!(skill.metadata().name, "system_info");
    }

    #[tokio::test]
    async fn test_execute_returns_info() {
        let skill = SysInfoSkill;
        let input = SkillInput {
            name: "system_info".into(),
            args: "{}".into(),
            user_id: "test".into(),
            platform: "test".into(),
        };
        let result = skill.execute(input).await;
        assert!(result.success);
        assert!(result.output.contains("CPU:"));
    }
}
