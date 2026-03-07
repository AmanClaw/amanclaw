//! AmanClaw Plugin SDK for Rust
//!
//! Use this crate to write WASM skill plugins in Rust.
//!
//! # Example
//! ```ignore
//! use amanclaw_plugin_sdk::*;
//!
//! pub fn metadata() -> SkillMetadata {
//!     SkillMetadata {
//!         name: "my_skill".into(),
//!         description: "Does something useful".into(),
//!         timeout_ms: 10000,
//!         version: "0.1.0".into(),
//!     }
//! }
//! ```

pub use serde_json;

// Re-export types that match the WIT interface
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillMetadata {
    pub name: String,
    pub description: String,
    pub timeout_ms: u32,
    pub version: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillInput {
    pub name: String,
    pub args: String,
    pub user_id: String,
    pub platform: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

impl SkillResult {
    pub fn ok(output: impl Into<String>) -> Self {
        Self { success: true, output: output.into(), error: None }
    }

    pub fn err(error: impl Into<String>) -> Self {
        Self { success: false, output: String::new(), error: Some(error.into()) }
    }
}

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub url: String,
    pub method: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
}
