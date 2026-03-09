use amanclaw_traits::config::SubAgentConfig;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, serde::Serialize)]
pub enum SubAgentStatus {
    Running,
    Completed { result: String },
    Failed { error: String },
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct SpawnRequest {
    pub agent_id: String,
    pub prompt: String,
    pub parent_session: String,
    pub depth: usize,
}

#[derive(Debug, Clone)]
pub struct SubAgent {
    pub id: String,
    pub agent_id: String,
    pub prompt: String,
    pub parent_session: String,
    pub depth: usize,
    pub status: SubAgentStatus,
}

#[derive(Clone)]
pub struct SubAgentManager {
    config: SubAgentConfig,
    agents: Arc<Mutex<HashMap<String, SubAgent>>>,
}

impl SubAgentManager {
    pub fn new(config: SubAgentConfig) -> Self {
        Self {
            config,
            agents: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn spawn(&self, request: SpawnRequest) -> Result<String, SubAgentError> {
        if !self.config.enabled {
            return Err(SubAgentError::Disabled);
        }

        if request.depth >= self.config.max_depth {
            return Err(SubAgentError::MaxDepthExceeded {
                max: self.config.max_depth,
            });
        }

        let mut agents = self.agents.lock().await;

        // Check global limit
        let active_count = agents
            .values()
            .filter(|a| matches!(a.status, SubAgentStatus::Running))
            .count();
        if active_count >= self.config.max_global {
            return Err(SubAgentError::GlobalLimitReached {
                max: self.config.max_global,
            });
        }

        // Check per-session limit
        let session_count = agents
            .values()
            .filter(|a| {
                a.parent_session == request.parent_session
                    && matches!(a.status, SubAgentStatus::Running)
            })
            .count();
        if session_count >= self.config.max_per_session {
            return Err(SubAgentError::SessionLimitReached {
                max: self.config.max_per_session,
            });
        }

        let id = uuid::Uuid::new_v4().to_string();
        let agent = SubAgent {
            id: id.clone(),
            agent_id: request.agent_id,
            prompt: request.prompt,
            parent_session: request.parent_session,
            depth: request.depth,
            status: SubAgentStatus::Running,
        };

        agents.insert(id.clone(), agent);
        Ok(id)
    }

    pub async fn complete(&self, id: &str, result: String) -> bool {
        let mut agents = self.agents.lock().await;
        if let Some(agent) = agents.get_mut(id) {
            if matches!(agent.status, SubAgentStatus::Running) {
                agent.status = SubAgentStatus::Completed { result };
                return true;
            }
        }
        false
    }

    pub async fn fail(&self, id: &str, error: String) -> bool {
        let mut agents = self.agents.lock().await;
        if let Some(agent) = agents.get_mut(id) {
            if matches!(agent.status, SubAgentStatus::Running) {
                agent.status = SubAgentStatus::Failed { error };
                return true;
            }
        }
        false
    }

    pub async fn cancel(&self, id: &str) -> bool {
        let mut agents = self.agents.lock().await;
        if let Some(agent) = agents.get_mut(id) {
            if matches!(agent.status, SubAgentStatus::Running) {
                agent.status = SubAgentStatus::Cancelled;
                return true;
            }
        }
        false
    }

    pub async fn cancel_all(&self, session: &str) -> usize {
        let mut agents = self.agents.lock().await;
        let mut count = 0;
        for agent in agents.values_mut() {
            if agent.parent_session == session && matches!(agent.status, SubAgentStatus::Running) {
                agent.status = SubAgentStatus::Cancelled;
                count += 1;
            }
        }
        count
    }

    pub async fn get(&self, id: &str) -> Option<SubAgent> {
        let agents = self.agents.lock().await;
        agents.get(id).cloned()
    }

    pub async fn list(&self, session: &str) -> Vec<SubAgent> {
        let agents = self.agents.lock().await;
        agents
            .values()
            .filter(|a| a.parent_session == session)
            .cloned()
            .collect()
    }

    pub async fn collect_results(&self, session: &str) -> Vec<SubAgent> {
        let agents = self.agents.lock().await;
        agents
            .values()
            .filter(|a| {
                a.parent_session == session
                    && !matches!(a.status, SubAgentStatus::Running)
            })
            .cloned()
            .collect()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SubAgentError {
    #[error("sub-agent spawning is disabled")]
    Disabled,
    #[error("max depth exceeded (max: {max})")]
    MaxDepthExceeded { max: usize },
    #[error("global sub-agent limit reached (max: {max})")]
    GlobalLimitReached { max: usize },
    #[error("per-session sub-agent limit reached (max: {max})")]
    SessionLimitReached { max: usize },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> SubAgentConfig {
        SubAgentConfig {
            enabled: true,
            max_per_session: 2,
            max_global: 3,
            max_depth: 2,
            default_timeout_secs: 60,
        }
    }

    #[tokio::test]
    async fn test_spawn_and_complete() {
        let mgr = SubAgentManager::new(test_config());
        let id = mgr
            .spawn(SpawnRequest {
                agent_id: "helper".into(),
                prompt: "do something".into(),
                parent_session: "sess1".into(),
                depth: 0,
            })
            .await
            .unwrap();

        let agent = mgr.get(&id).await.unwrap();
        assert!(matches!(agent.status, SubAgentStatus::Running));

        mgr.complete(&id, "done".into()).await;
        let agent = mgr.get(&id).await.unwrap();
        assert!(matches!(agent.status, SubAgentStatus::Completed { .. }));
    }

    #[tokio::test]
    async fn test_session_limit() {
        let mgr = SubAgentManager::new(test_config());
        let req = || SpawnRequest {
            agent_id: "helper".into(),
            prompt: "task".into(),
            parent_session: "sess1".into(),
            depth: 0,
        };

        mgr.spawn(req()).await.unwrap();
        mgr.spawn(req()).await.unwrap();

        let err = mgr.spawn(req()).await.unwrap_err();
        assert!(matches!(err, SubAgentError::SessionLimitReached { max: 2 }));
    }

    #[tokio::test]
    async fn test_global_limit() {
        let mgr = SubAgentManager::new(test_config());

        for i in 0..3 {
            mgr.spawn(SpawnRequest {
                agent_id: "helper".into(),
                prompt: "task".into(),
                parent_session: format!("sess{}", i),
                depth: 0,
            })
            .await
            .unwrap();
        }

        let err = mgr
            .spawn(SpawnRequest {
                agent_id: "helper".into(),
                prompt: "task".into(),
                parent_session: "sess99".into(),
                depth: 0,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, SubAgentError::GlobalLimitReached { max: 3 }));
    }

    #[tokio::test]
    async fn test_max_depth() {
        let mgr = SubAgentManager::new(test_config());
        let err = mgr
            .spawn(SpawnRequest {
                agent_id: "helper".into(),
                prompt: "task".into(),
                parent_session: "sess1".into(),
                depth: 2,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, SubAgentError::MaxDepthExceeded { max: 2 }));
    }

    #[tokio::test]
    async fn test_cancel_and_cancel_all() {
        let mgr = SubAgentManager::new(test_config());
        let id1 = mgr
            .spawn(SpawnRequest {
                agent_id: "a".into(),
                prompt: "t".into(),
                parent_session: "sess1".into(),
                depth: 0,
            })
            .await
            .unwrap();
        let _id2 = mgr
            .spawn(SpawnRequest {
                agent_id: "b".into(),
                prompt: "t".into(),
                parent_session: "sess1".into(),
                depth: 0,
            })
            .await
            .unwrap();

        // Cancel one
        assert!(mgr.cancel(&id1).await);
        let agent = mgr.get(&id1).await.unwrap();
        assert!(matches!(agent.status, SubAgentStatus::Cancelled));

        // Cancel all remaining in session
        let cancelled = mgr.cancel_all("sess1").await;
        assert_eq!(cancelled, 1);
    }

    #[tokio::test]
    async fn test_disabled() {
        let mut config = test_config();
        config.enabled = false;
        let mgr = SubAgentManager::new(config);

        let err = mgr
            .spawn(SpawnRequest {
                agent_id: "helper".into(),
                prompt: "task".into(),
                parent_session: "sess1".into(),
                depth: 0,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, SubAgentError::Disabled));
    }
}
