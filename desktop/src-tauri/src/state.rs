use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppMode {
    Local,
    Remote { url: String, token: String },
}

#[derive(Debug)]
pub struct AppState {
    pub mode: AppMode,
    pub bot_running: bool,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            mode: AppMode::Local,
            bot_running: false,
        }
    }
}
