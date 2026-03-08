use amanclaw_traits::agent::AgentProfile;
use amanclaw_traits::config::{RoutingRule, RoutingMatch};
use amanclaw_traits::message::IncomingMessage;
use std::collections::HashMap;

/// Routes incoming messages to agent profiles based on config rules.
pub struct AgentRouter {
    profiles: HashMap<String, AgentProfile>,
    rules: Vec<RoutingRule>,
    default_agent_id: String,
}

impl AgentRouter {
    pub fn new(
        profiles: HashMap<String, AgentProfile>,
        rules: Vec<RoutingRule>,
        default_agent_id: String,
    ) -> Self {
        Self { profiles, rules, default_agent_id }
    }

    /// Resolve which agent profile should handle this message.
    pub fn resolve(&self, msg: &IncomingMessage) -> AgentProfile {
        for rule in &self.rules {
            if self.matches(&rule.match_criteria, msg) {
                if let Some(profile) = self.profiles.get(&rule.agent) {
                    return profile.clone();
                }
            }
        }

        self.profiles
            .get(&self.default_agent_id)
            .cloned()
            .unwrap_or_else(AgentProfile::default_agent)
    }

    fn matches(&self, criteria: &RoutingMatch, msg: &IncomingMessage) -> bool {
        if let Some(ref platform) = criteria.platform {
            if platform != &msg.platform {
                return false;
            }
        }
        if let Some(ref topic_id) = criteria.topic_id {
            if msg.topic_id.as_deref() != Some(topic_id) {
                return false;
            }
        }
        if let Some(ref channel_id) = criteria.channel_id {
            if msg.channel_context.as_deref() != Some(channel_id) {
                return false;
            }
        }
        if let Some(ref group_id) = criteria.group_id {
            if &msg.chat_id != group_id {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use amanclaw_traits::agent::ContextConfig;

    fn make_msg(platform: &str, topic_id: Option<&str>, chat_id: &str, channel_ctx: Option<&str>) -> IncomingMessage {
        IncomingMessage {
            user_id: "u1".into(),
            chat_id: chat_id.into(),
            platform: platform.into(),
            text: "test".into(),
            username: None,
            first_name: None,
            is_group: false,
            image_data: None,
            reply_to: None,
            topic_id: topic_id.map(Into::into),
            channel_context: channel_ctx.map(Into::into),
        }
    }

    fn make_profile(id: &str) -> AgentProfile {
        AgentProfile {
            id: id.into(),
            name: id.into(),
            system_prompt: format!("{} prompt", id),
            allowed_skills: vec![],
            llm_override: None,
            soul_file: None,
            memory_namespace: id.into(),
            context: ContextConfig::default(),
        }
    }

    #[test]
    fn test_matches_platform_and_topic() {
        let profiles = HashMap::from([("ustazbot".into(), make_profile("ustazbot"))]);
        let rules = vec![RoutingRule {
            match_criteria: RoutingMatch {
                platform: Some("telegram".into()),
                topic_id: Some("123".into()),
                channel_id: None,
                group_id: None,
            },
            agent: "ustazbot".into(),
        }];

        let router = AgentRouter::new(profiles, rules, "default".into());
        let profile = router.resolve(&make_msg("telegram", Some("123"), "c1", None));
        assert_eq!(profile.id, "ustazbot");
    }

    #[test]
    fn test_falls_back_to_default() {
        let router = AgentRouter::new(HashMap::new(), vec![], "default".into());
        let profile = router.resolve(&make_msg("telegram", None, "c1", None));
        assert_eq!(profile.id, "default");
    }

    #[test]
    fn test_matches_group_id() {
        let profiles = HashMap::from([("halalbot".into(), make_profile("halalbot"))]);
        let rules = vec![RoutingRule {
            match_criteria: RoutingMatch {
                platform: Some("telegram".into()),
                topic_id: None,
                channel_id: None,
                group_id: Some("group789".into()),
            },
            agent: "halalbot".into(),
        }];

        let router = AgentRouter::new(profiles, rules, "default".into());
        let profile = router.resolve(&make_msg("telegram", None, "group789", None));
        assert_eq!(profile.id, "halalbot");
    }

    #[test]
    fn test_no_match_wrong_platform() {
        let profiles = HashMap::from([("ustazbot".into(), make_profile("ustazbot"))]);
        let rules = vec![RoutingRule {
            match_criteria: RoutingMatch {
                platform: Some("telegram".into()),
                topic_id: None,
                channel_id: None,
                group_id: None,
            },
            agent: "ustazbot".into(),
        }];

        let router = AgentRouter::new(profiles, rules, "default".into());
        let profile = router.resolve(&make_msg("discord", None, "c1", None));
        assert_eq!(profile.id, "default");
    }

    #[test]
    fn test_matches_channel_context() {
        let profiles = HashMap::from([("devbot".into(), make_profile("devbot"))]);
        let rules = vec![RoutingRule {
            match_criteria: RoutingMatch {
                platform: Some("discord".into()),
                topic_id: None,
                channel_id: Some("dev-chat".into()),
                group_id: None,
            },
            agent: "devbot".into(),
        }];

        let router = AgentRouter::new(profiles, rules, "default".into());
        let profile = router.resolve(&make_msg("discord", None, "c1", Some("dev-chat")));
        assert_eq!(profile.id, "devbot");
    }
}
