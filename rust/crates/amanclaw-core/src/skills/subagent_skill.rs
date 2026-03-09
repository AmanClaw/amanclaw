use crate::subagent::{SpawnRequest, SubAgentManager};
use amanclaw_traits::skill::{Skill, SkillInput, SkillMetadata, SkillResult};
use std::sync::Arc;

pub struct SubAgentSkill {
    manager: Arc<SubAgentManager>,
}

impl SubAgentSkill {
    pub fn new(manager: Arc<SubAgentManager>) -> Self {
        Self { manager }
    }
}

#[async_trait::async_trait]
impl Skill for SubAgentSkill {
    fn metadata(&self) -> SkillMetadata {
        SkillMetadata {
            name: "subagent".into(),
            description: "Spawn, check, and cancel sub-agents for parallel task execution".into(),
            timeout_ms: 5000,
            version: "0.1.0".into(),
        }
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["spawn", "check", "cancel"],
                    "description": "Action to perform"
                },
                "agent_id": {
                    "type": "string",
                    "description": "Agent profile ID to use (for spawn)"
                },
                "prompt": {
                    "type": "string",
                    "description": "Task prompt for the sub-agent (for spawn)"
                },
                "subagent_id": {
                    "type": "string",
                    "description": "Sub-agent ID (for check/cancel)"
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, input: SkillInput) -> SkillResult {
        let args: serde_json::Value = match serde_json::from_str(&input.args) {
            Ok(v) => v,
            Err(e) => {
                return SkillResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Invalid args: {e}")),
                };
            }
        };

        let action = args["action"].as_str().unwrap_or("");

        match action {
            "spawn" => self.handle_spawn(&args, &input).await,
            "check" => self.handle_check(&args, &input).await,
            "cancel" => self.handle_cancel(&args).await,
            _ => SkillResult {
                success: false,
                output: String::new(),
                error: Some(format!("Unknown action: {action}")),
            },
        }
    }
}

impl SubAgentSkill {
    async fn handle_spawn(&self, args: &serde_json::Value, input: &SkillInput) -> SkillResult {
        let agent_id = args["agent_id"].as_str().unwrap_or("default").to_string();
        let prompt = match args["prompt"].as_str() {
            Some(p) => p.to_string(),
            None => {
                return SkillResult {
                    success: false,
                    output: String::new(),
                    error: Some("Missing 'prompt' for spawn action".into()),
                };
            }
        };

        let request = SpawnRequest {
            agent_id,
            prompt,
            parent_session: input.user_id.clone(),
            depth: 0,
        };

        match self.manager.spawn(request).await {
            Ok(id) => SkillResult {
                success: true,
                output: serde_json::json!({
                    "subagent_id": id,
                    "status": "spawned"
                })
                .to_string(),
                error: None,
            },
            Err(e) => SkillResult {
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
            },
        }
    }

    async fn handle_check(&self, args: &serde_json::Value, input: &SkillInput) -> SkillResult {
        if let Some(id) = args["subagent_id"].as_str() {
            // Check specific sub-agent
            match self.manager.get(id).await {
                Some(agent) => SkillResult {
                    success: true,
                    output: serde_json::json!({
                        "subagent_id": agent.id,
                        "agent_id": agent.agent_id,
                        "status": format!("{:?}", agent.status),
                    })
                    .to_string(),
                    error: None,
                },
                None => SkillResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Sub-agent '{id}' not found")),
                },
            }
        } else {
            // List all sub-agents for session
            let agents = self.manager.list(&input.user_id).await;
            let list: Vec<serde_json::Value> = agents
                .iter()
                .map(|a| {
                    serde_json::json!({
                        "subagent_id": a.id,
                        "agent_id": a.agent_id,
                        "status": format!("{:?}", a.status),
                    })
                })
                .collect();
            SkillResult {
                success: true,
                output: serde_json::json!({ "subagents": list }).to_string(),
                error: None,
            }
        }
    }

    async fn handle_cancel(&self, args: &serde_json::Value) -> SkillResult {
        let id = match args["subagent_id"].as_str() {
            Some(id) => id,
            None => {
                return SkillResult {
                    success: false,
                    output: String::new(),
                    error: Some("Missing 'subagent_id' for cancel action".into()),
                };
            }
        };

        if self.manager.cancel(id).await {
            SkillResult {
                success: true,
                output: serde_json::json!({
                    "subagent_id": id,
                    "status": "cancelled"
                })
                .to_string(),
                error: None,
            }
        } else {
            SkillResult {
                success: false,
                output: String::new(),
                error: Some(format!("Sub-agent '{id}' not found or not running")),
            }
        }
    }
}
