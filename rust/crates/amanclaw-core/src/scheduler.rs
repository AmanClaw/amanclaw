use amanclaw_traits::config::CronJobConfig;
use amanclaw_traits::message::{IncomingMessage, OutgoingMessage};
use anyhow::Result;
use std::collections::HashMap;
use std::str::FromStr;
use tokio::sync::mpsc;

/// Events produced by the scheduler (shared with webhooks).
#[derive(Debug)]
pub enum SchedulerEvent {
    /// Direct message ready to send (no LLM processing).
    SendMessage(OutgoingMessage),
    /// Inject into pipeline for agent processing.
    InjectMessage(IncomingMessage),
}

pub struct Scheduler {
    tx: mpsc::Sender<SchedulerEvent>,
    handles: HashMap<String, tokio::task::JoinHandle<()>>,
}

impl Scheduler {
    pub fn new(tx: mpsc::Sender<SchedulerEvent>) -> Self {
        Self { tx, handles: HashMap::new() }
    }

    pub fn start_jobs(&mut self, jobs: &HashMap<String, CronJobConfig>, default_tz: &str) {
        for (id, job) in jobs {
            if !job.enabled {
                tracing::info!(job = %id, "Cron job disabled, skipping");
                continue;
            }

            let schedule = match cron::Schedule::from_str(&job.schedule) {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!(job = %id, error = %e, "Invalid cron expression");
                    continue;
                }
            };

            let tz_str = job.timezone.as_deref().unwrap_or(default_tz);
            let tz: chrono_tz::Tz = tz_str.parse().unwrap_or(chrono_tz::UTC);
            let tx = self.tx.clone();
            let job_id = id.clone();
            let job_clone = job.clone();

            let handle = tokio::spawn(async move {
                loop {
                    let now = chrono::Utc::now().with_timezone(&tz);
                    let next = schedule.upcoming(tz).next();
                    if let Some(next_time) = next {
                        let wait = (next_time - now).to_std().unwrap_or(std::time::Duration::from_secs(1));
                        tokio::time::sleep(wait).await;

                        if let Err(e) = Self::fire_job(&job_id, &job_clone, &tx).await {
                            tracing::error!(job = %job_id, error = %e, "Cron job failed");
                        }
                    } else {
                        break;
                    }
                }
            });

            self.handles.insert(id.clone(), handle);
            tracing::info!(job = %id, schedule = %job.schedule, "Cron job scheduled");
        }
    }

    async fn fire_job(
        job_id: &str,
        job: &CronJobConfig,
        tx: &mpsc::Sender<SchedulerEvent>,
    ) -> Result<()> {
        for target in &job.targets {
            match job.job_type.as_str() {
                "direct_message" => {
                    let text = job.template.clone().unwrap_or_default();
                    tx.send(SchedulerEvent::SendMessage(OutgoingMessage {
                        chat_id: target.chat_id.clone(),
                        text,
                        parse_mode: None,
                        reply_to: None,
                        platform: Some(target.platform.clone()),
                        topic_id: target.topic_id.clone(),
                    })).await?;
                }
                "skill_invocation" => {
                    let skill = job.skill.clone().unwrap_or_default();
                    let input = job.input.clone().unwrap_or_default();
                    let synthetic = format!("/{} {}", skill, input);
                    tx.send(SchedulerEvent::InjectMessage(IncomingMessage {
                        user_id: format!("cron:{}", job_id),
                        chat_id: target.chat_id.clone(),
                        platform: target.platform.clone(),
                        text: synthetic,
                        username: None,
                        first_name: None,
                        is_group: false,
                        image_data: None,
                        reply_to: None,
                        topic_id: target.topic_id.clone(),
                        channel_context: None,
                        is_cron: true,
                        is_webhook: false,
                        is_subagent: false,
                    })).await?;
                }
                "agent_prompt" => {
                    let prompt = job.prompt.clone().unwrap_or_default();
                    tx.send(SchedulerEvent::InjectMessage(IncomingMessage {
                        user_id: format!("cron:{}", job_id),
                        chat_id: target.chat_id.clone(),
                        platform: target.platform.clone(),
                        text: prompt,
                        username: None,
                        first_name: None,
                        is_group: false,
                        image_data: None,
                        reply_to: None,
                        topic_id: target.topic_id.clone(),
                        channel_context: None,
                        is_cron: true,
                        is_webhook: false,
                        is_subagent: false,
                    })).await?;
                }
                other => {
                    tracing::warn!(job = %job_id, job_type = %other, "Unknown cron job type");
                }
            }
        }
        tracing::info!(job = %job_id, "Cron job fired");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use amanclaw_traits::config::CronTargetConfig;

    #[tokio::test]
    async fn test_direct_message_event() {
        let (tx, mut rx) = mpsc::channel(16);
        let target = CronTargetConfig {
            platform: "telegram".into(),
            chat_id: "12345".into(),
            topic_id: None,
        };
        let job = CronJobConfig {
            name: "Test".into(),
            schedule: "* * * * * *".into(), // Every second
            timezone: None,
            job_type: "direct_message".into(),
            skill: None,
            input: None,
            prompt: None,
            template: Some("Hello test".into()),
            targets: vec![target],
            agent: None,
            enabled: true,
        };

        let mut scheduler = Scheduler::new(tx);
        let mut jobs = HashMap::new();
        jobs.insert("test".into(), job);
        scheduler.start_jobs(&jobs, "UTC");

        // Wait up to 3 seconds for an event
        let event = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            rx.recv()
        ).await;

        assert!(event.is_ok(), "Should receive a scheduler event within 3 seconds");
        match event.unwrap().unwrap() {
            SchedulerEvent::SendMessage(msg) => {
                assert_eq!(msg.text, "Hello test");
                assert_eq!(msg.chat_id, "12345");
            }
            _ => panic!("Expected SendMessage"),
        }
    }

    #[tokio::test]
    async fn test_agent_prompt_event() {
        let (tx, mut rx) = mpsc::channel(16);
        let target = CronTargetConfig {
            platform: "telegram".into(),
            chat_id: "12345".into(),
            topic_id: None,
        };
        let job = CronJobConfig {
            name: "Quran Daily".into(),
            schedule: "* * * * * *".into(),
            timezone: None,
            job_type: "agent_prompt".into(),
            skill: None,
            input: None,
            prompt: Some("Share a verse".into()),
            template: None,
            targets: vec![target],
            agent: Some("ustazbot".into()),
            enabled: true,
        };

        let mut scheduler = Scheduler::new(tx);
        let mut jobs = HashMap::new();
        jobs.insert("quran".into(), job);
        scheduler.start_jobs(&jobs, "UTC");

        let event = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            rx.recv()
        ).await;

        assert!(event.is_ok());
        match event.unwrap().unwrap() {
            SchedulerEvent::InjectMessage(msg) => {
                assert_eq!(msg.text, "Share a verse");
                assert!(msg.is_cron);
                assert!(msg.user_id.starts_with("cron:"));
            }
            _ => panic!("Expected InjectMessage"),
        }
    }
}
