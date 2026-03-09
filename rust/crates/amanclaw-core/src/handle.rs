use amanclaw_traits::message::IncomingMessage;
use amanclaw_traits::skill::SkillMetadata;
use crate::scheduler::SchedulerEvent;
use tokio::sync::{mpsc, oneshot, watch};
use std::time::Instant;

/// Commands sent to the engine actor.
pub enum EngineCommand {
    /// Process an incoming chat message through the pipeline.
    ProcessMessage(IncomingMessage),
    /// Forward a scheduler event.
    SchedulerEvent(SchedulerEvent),
    /// Query engine status.
    GetStatus(oneshot::Sender<EngineStatus>),
    /// Query available skills.
    GetSkills(oneshot::Sender<Vec<SkillMetadata>>),
    /// Request graceful shutdown.
    Shutdown(oneshot::Sender<()>),
}

/// Engine runtime status.
#[derive(Debug, Clone)]
pub enum EngineStatus {
    Stopped,
    Starting,
    Running {
        started_at: Instant,
        messages_processed: u64,
    },
    Error(String),
}

impl EngineStatus {
    pub fn is_running(&self) -> bool {
        matches!(self, Self::Running { .. })
    }
}

/// Cheap, cloneable handle to communicate with the engine actor.
#[derive(Clone)]
pub struct EngineHandle {
    cmd_tx: mpsc::Sender<EngineCommand>,
    status_rx: watch::Receiver<EngineStatus>,
}

impl EngineHandle {
    pub fn new(
        cmd_tx: mpsc::Sender<EngineCommand>,
        status_rx: watch::Receiver<EngineStatus>,
    ) -> Self {
        Self { cmd_tx, status_rx }
    }

    /// Send a message for processing.
    pub async fn send_message(&self, msg: IncomingMessage) -> anyhow::Result<()> {
        self.cmd_tx
            .send(EngineCommand::ProcessMessage(msg))
            .await
            .map_err(|_| anyhow::anyhow!("engine actor stopped"))
    }

    /// Get current status (non-blocking snapshot).
    pub fn status(&self) -> EngineStatus {
        self.status_rx.borrow().clone()
    }

    /// Wait for a status change.
    pub async fn wait_for_status_change(&mut self) -> EngineStatus {
        let _ = self.status_rx.changed().await;
        self.status_rx.borrow().clone()
    }

    /// Query available skills from the actor.
    pub async fn skills(&self) -> anyhow::Result<Vec<SkillMetadata>> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(EngineCommand::GetSkills(tx))
            .await
            .map_err(|_| anyhow::anyhow!("engine actor stopped"))?;
        rx.await
            .map_err(|_| anyhow::anyhow!("engine actor dropped response"))
    }

    /// Request graceful shutdown and wait for completion.
    pub async fn shutdown(&self) -> anyhow::Result<()> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(EngineCommand::Shutdown(tx))
            .await
            .map_err(|_| anyhow::anyhow!("engine already stopped"))?;
        rx.await
            .map_err(|_| anyhow::anyhow!("engine dropped shutdown response"))
    }
}
