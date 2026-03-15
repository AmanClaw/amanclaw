use crate::state::{AppState, LogEntry};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing_subscriber::Layer;

/// A tracing layer that captures log events into AppState's log buffer.
pub struct AppLogLayer {
    state: Arc<RwLock<AppState>>,
}

impl AppLogLayer {
    pub fn new(state: Arc<RwLock<AppState>>) -> Self {
        Self { state }
    }
}

impl<S: tracing::Subscriber> Layer<S> for AppLogLayer {
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        let meta = event.metadata();
        let level = meta.level().to_string();
        let target = meta.target().to_string();

        // Extract message from fields
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);

        let entry = LogEntry {
            timestamp: chrono::Local::now().format("%H:%M:%S%.3f").to_string(),
            level,
            target,
            message: visitor.message,
        };

        let state = self.state.clone();
        // Use try_write to avoid blocking the tracing pipeline
        if let Ok(mut st) = state.try_write() {
            st.push_log(entry);
        }
    }
}

#[derive(Default)]
struct MessageVisitor {
    message: String,
}

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
        } else if self.message.is_empty() {
            self.message = format!("{}={:?}", field.name(), value);
        } else {
            self.message.push_str(&format!(" {}={:?}", field.name(), value));
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        } else if self.message.is_empty() {
            self.message = format!("{}={}", field.name(), value);
        } else {
            self.message.push_str(&format!(" {}={}", field.name(), value));
        }
    }
}
