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
        Self {
            success: true,
            output: output.into(),
            error: None,
        }
    }

    pub fn err(error: impl Into<String>) -> Self {
        Self {
            success: false,
            output: String::new(),
            error: Some(error.into()),
        }
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

// --- WASM ABI helpers ---
// Use these macros to easily create WASM-compatible plugins.

/// Register a WASM skill plugin.
///
/// # Example
/// ```ignore
/// use amanclaw_plugin_sdk::*;
///
/// amanclaw_plugin!(
///     metadata: SkillMetadata {
///         name: "my_skill".into(),
///         description: "Does something".into(),
///         timeout_ms: 10000,
///         version: "0.1.0".into(),
///     },
///     parameters: r#"{"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}"#,
///     execute: |input: SkillInput| -> SkillResult {
///         SkillResult::ok(format!("Got: {}", input.args))
///     }
/// );
/// ```
#[macro_export]
macro_rules! amanclaw_plugin {
    (
        metadata: $meta:expr,
        parameters: $params:expr,
        execute: |$input:ident : SkillInput| -> SkillResult $body:block
    ) => {
        static METADATA_JSON: std::sync::LazyLock<String> =
            std::sync::LazyLock::new(|| $crate::serde_json::to_string(&$meta).unwrap());

        static PARAMS_JSON: &str = $params;

        #[unsafe(no_mangle)]
        pub extern "C" fn alloc(size: i32) -> *mut u8 {
            let layout = std::alloc::Layout::from_size_align(size as usize, 1).unwrap();
            unsafe { std::alloc::alloc(layout) }
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn dealloc(ptr: *mut u8, size: i32) {
            let layout = std::alloc::Layout::from_size_align(size as usize, 1).unwrap();
            unsafe { std::alloc::dealloc(ptr, layout) }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn metadata() -> *const u8 {
            METADATA_JSON.as_ptr()
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn parameters() -> *const u8 {
            PARAMS_JSON.as_ptr()
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn execute(ptr: i32, len: i32) -> *const u8 {
            let input_bytes = unsafe { std::slice::from_raw_parts(ptr as *const u8, len as usize) };
            let $input: $crate::SkillInput = match $crate::serde_json::from_slice(input_bytes) {
                Ok(i) => i,
                Err(e) => {
                    let result = $crate::SkillResult::err(format!("Invalid input: {}", e));
                    let json = $crate::serde_json::to_string(&result).unwrap();
                    let leaked = json.into_bytes().leak();
                    return leaked.as_ptr();
                }
            };

            let result: $crate::SkillResult = $body;
            let json = $crate::serde_json::to_string(&result).unwrap();
            // Null-terminate
            let mut bytes = json.into_bytes();
            bytes.push(0);
            let leaked = bytes.leak();
            leaked.as_ptr()
        }
    };
}
