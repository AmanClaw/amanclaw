//! Full WASM plugin instantiation and execution.
//!
//! Uses a simple JSON-based ABI where WASM modules export:
//! - `alloc(size) -> ptr` — allocate memory for input
//! - `dealloc(ptr, size)` — free allocated memory
//! - `metadata() -> ptr` — returns JSON string pointer (null-terminated)
//! - `parameters() -> ptr` — returns JSON schema string pointer
//! - `execute(ptr, len) -> ptr` — takes JSON SkillInput, returns JSON SkillResult
//!
//! This avoids the complexity of the component model while still providing
//! a clean, language-agnostic plugin interface.

use amanclaw_traits::skill::{Skill, SkillInput, SkillMetadata, SkillResult};
use anyhow::{Context, Result};
use std::path::Path;
use std::sync::Arc;
use wasmtime::*;
use wasmtime::{StoreLimits, StoreLimitsBuilder};

use crate::sandbox::SandboxConfig;

/// A loaded WASM plugin that implements the Skill trait.
pub struct WasmSkill {
    name: String,
    description: String,
    version: String,
    timeout_ms: u32,
    parameters_json: String,
    engine: Engine,
    module: Module,
    sandbox: SandboxConfig,
}

impl std::fmt::Debug for WasmSkill {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmSkill")
            .field("name", &self.name)
            .field("version", &self.version)
            .finish()
    }
}

/// Read a null-terminated string from WASM memory starting at `ptr`.
fn read_cstring(memory: &Memory, store: &mut Store<StoreLimits>, ptr: i32) -> Result<String> {
    let data = memory.data(&store);
    let start = ptr as usize;
    let mut end = start;
    while end < data.len() && data[end] != 0 {
        end += 1;
    }
    let bytes = &data[start..end];
    Ok(String::from_utf8_lossy(bytes).into_owned())
}

/// Write bytes into WASM memory via the alloc export, returning the pointer.
fn write_bytes(
    alloc_fn: &TypedFunc<i32, i32>,
    memory: &Memory,
    store: &mut Store<StoreLimits>,
    data: &[u8],
) -> Result<(i32, i32)> {
    let len = data.len() as i32;
    let ptr = alloc_fn.call(&mut *store, len)?;
    memory.write(&mut *store, ptr as usize, data)?;
    Ok((ptr, len))
}

/// Load a single WASM module and extract its metadata.
pub fn load_wasm_skill(
    wasm_path: &Path,
    sandbox: SandboxConfig,
) -> Result<WasmSkill> {
    let mut config = Config::new();
    config.epoch_interruption(true);
    config.consume_fuel(true);

    let engine = Engine::new(&config)?;
    let module = Module::from_file(&engine, wasm_path)
        .with_context(|| format!("Failed to load WASM module: {}", wasm_path.display()))?;

    // Create store with resource limits
    let limits = StoreLimitsBuilder::new()
        .memory_size(sandbox.max_memory_bytes)
        .table_elements(10_000)
        .build();
    let mut store = Store::new(&engine, limits);
    store.limiter(|s| s as &mut dyn wasmtime::ResourceLimiter);
    store.set_epoch_deadline(100); // generous deadline for init
    store.set_fuel(sandbox.fuel_limit)?;

    let mut linker = wasmtime::Linker::new(&engine);

    // Provide minimal WASI-like imports if the module needs them
    // Many modules compiled from Rust/C need at least fd_write for panic messages
    provide_stub_imports(&mut linker, &module)?;

    let instance = linker.instantiate(&mut store, &module)
        .with_context(|| "Failed to instantiate WASM module")?;

    let memory = instance.get_memory(&mut store, "memory")
        .ok_or_else(|| anyhow::anyhow!("WASM module has no 'memory' export"))?;

    // Call metadata()
    let metadata_fn = instance.get_typed_func::<(), i32>(&mut store, "metadata")
        .with_context(|| "WASM module missing 'metadata' export")?;
    let meta_ptr = metadata_fn.call(&mut store, ())?;
    let meta_json = read_cstring(&memory, &mut store, meta_ptr)?;
    let metadata: SkillMetadata = serde_json::from_str(&meta_json)
        .with_context(|| format!("Invalid metadata JSON from plugin: {}", meta_json))?;

    // Call parameters()
    let params_fn = instance.get_typed_func::<(), i32>(&mut store, "parameters")
        .with_context(|| "WASM module missing 'parameters' export")?;
    let params_ptr = params_fn.call(&mut store, ())?;
    let parameters_json = read_cstring(&memory, &mut store, params_ptr)?;

    tracing::info!(
        name = %metadata.name,
        version = %metadata.version,
        path = %wasm_path.display(),
        "Loaded WASM plugin"
    );

    Ok(WasmSkill {
        name: metadata.name.clone(),
        description: metadata.description.clone(),
        version: metadata.version.clone(),
        timeout_ms: metadata.timeout_ms,
        parameters_json,
        engine,
        module,
        sandbox,
    })
}

/// Provide stub imports for common WASI functions that modules might need.
fn provide_stub_imports(linker: &mut wasmtime::Linker<StoreLimits>, module: &Module) -> Result<()> {
    for import in module.imports() {
        let module_name = import.module();
        let name = import.name();

        // Only stub functions we haven't already defined
        let check_limits = StoreLimitsBuilder::new().build();
        if linker.get(&mut Store::new(linker.engine(), check_limits), module_name, name).is_some() {
            continue;
        }

        match import.ty() {
            ExternType::Func(func_ty) => {
                let params: Vec<ValType> = func_ty.params().collect();
                let results: Vec<ValType> = func_ty.results().collect();

                // Create appropriate stub based on the function type
                let stub_ty = FuncType::new(linker.engine(), params.iter().cloned(), results.iter().cloned());
                let m = module_name.to_string();
                let n = name.to_string();

                linker.func_new(module_name, name, stub_ty, move |_caller, _params, results| {
                    tracing::debug!(module = %m, func = %n, "Stub WASI call");
                    // Return zeros for all result types
                    for result in results.iter_mut() {
                        *result = Val::I32(0);
                    }
                    Ok(())
                })?;
            }
            _ => {}
        }
    }
    Ok(())
}

#[async_trait::async_trait]
impl Skill for WasmSkill {
    fn metadata(&self) -> SkillMetadata {
        SkillMetadata {
            name: self.name.clone(),
            description: self.description.clone(),
            timeout_ms: self.timeout_ms,
            version: self.version.clone(),
        }
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::from_str(&self.parameters_json)
            .unwrap_or_else(|_| serde_json::json!({"type": "object", "properties": {}}))
    }

    async fn execute(&self, input: SkillInput) -> SkillResult {
        // Run WASM execution on a blocking thread (it's synchronous)
        let engine = self.engine.clone();
        let module = self.module.clone();
        let sandbox = self.sandbox.clone();

        let result = tokio::task::spawn_blocking(move || {
            execute_wasm(&engine, &module, &input, &sandbox)
        }).await;

        match result {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => SkillResult {
                success: false,
                output: String::new(),
                error: Some(format!("WASM execution error: {}", e)),
            },
            Err(e) => SkillResult {
                success: false,
                output: String::new(),
                error: Some(format!("WASM task panicked: {}", e)),
            },
        }
    }
}

/// Execute a WASM module's `execute` function with the given input.
fn execute_wasm(
    engine: &Engine,
    module: &Module,
    input: &SkillInput,
    sandbox: &SandboxConfig,
) -> Result<SkillResult> {
    let limits = StoreLimitsBuilder::new()
        .memory_size(sandbox.max_memory_bytes)
        .table_elements(10_000)
        .build();
    let mut store = Store::new(engine, limits);
    store.limiter(|s| s as &mut dyn wasmtime::ResourceLimiter);

    // Set fuel budget for CPU limiting
    if sandbox.fuel_limit > 0 {
        store.set_fuel(sandbox.fuel_limit)?;
    }

    // Set epoch deadline for timeout
    let timeout = sandbox.timeout;
    let epoch_ticks = (timeout.as_millis() / 10).max(1) as u64;
    store.set_epoch_deadline(epoch_ticks);

    let engine_clone = engine.clone();
    std::thread::spawn(move || {
        std::thread::sleep(timeout);
        engine_clone.increment_epoch();
    });

    let mut linker = wasmtime::Linker::new(engine);
    provide_stub_imports(&mut linker, module)?;

    let instance = linker.instantiate(&mut store, module)?;
    let memory = instance.get_memory(&mut store, "memory")
        .ok_or_else(|| anyhow::anyhow!("No memory export"))?;

    let alloc_fn = instance.get_typed_func::<i32, i32>(&mut store, "alloc")
        .with_context(|| "Missing 'alloc' export")?;

    let execute_fn = instance.get_typed_func::<(i32, i32), i32>(&mut store, "execute")
        .with_context(|| "Missing 'execute' export")?;

    // Serialize input to JSON and write to WASM memory
    let input_json = serde_json::to_string(input)?;
    let (ptr, len) = write_bytes(&alloc_fn, &memory, &mut store, input_json.as_bytes())?;

    // Call execute
    let result_ptr = execute_fn.call(&mut store, (ptr, len))?;
    let result_json = read_cstring(&memory, &mut store, result_ptr)?;

    let result: SkillResult = serde_json::from_str(&result_json)
        .with_context(|| format!("Invalid result JSON from plugin: {}", result_json))?;

    Ok(result)
}

/// Build a SandboxConfig from memory limit (in MB) and fuel limit.
pub fn sandbox_from_limits(memory_limit_mb: u64, fuel_limit: u64) -> SandboxConfig {
    SandboxConfig {
        max_memory_bytes: (memory_limit_mb as usize) * 1024 * 1024,
        fuel_limit,
        ..SandboxConfig::default()
    }
}

/// Discover and load all .wasm plugins from a directory.
pub fn load_all_plugins(plugin_dir: &Path, sandbox: SandboxConfig) -> Vec<Arc<dyn Skill>> {
    let mut skills: Vec<Arc<dyn Skill>> = Vec::new();

    if !plugin_dir.exists() {
        tracing::warn!(dir = %plugin_dir.display(), "Plugin directory does not exist");
        return skills;
    }

    let entries = match std::fs::read_dir(plugin_dir) {
        Ok(e) => e,
        Err(e) => {
            tracing::error!(error = %e, "Failed to read plugin directory");
            return skills;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "wasm") {
            let sandbox = sandbox.clone();
            match load_wasm_skill(&path, sandbox) {
                Ok(skill) => {
                    tracing::info!(name = %skill.name, "Loaded WASM skill");
                    skills.push(Arc::new(skill));
                }
                Err(e) => {
                    tracing::error!(
                        path = %path.display(),
                        error = %e,
                        "Failed to load WASM plugin"
                    );
                }
            }
        }
    }

    skills
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_default() {
        let sandbox = SandboxConfig::default();
        assert_eq!(sandbox.timeout.as_secs(), 30);
    }

    #[test]
    fn test_load_nonexistent_plugin() {
        let result = load_wasm_skill(
            Path::new("/tmp/nonexistent.wasm"),
            SandboxConfig::default(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_load_all_plugins_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let skills = load_all_plugins(dir.path(), SandboxConfig::default());
        assert!(skills.is_empty());
    }

    #[test]
    fn test_load_all_plugins_nonexistent_dir() {
        let skills = load_all_plugins(Path::new("/tmp/nonexistent-plugins-dir"), SandboxConfig::default());
        assert!(skills.is_empty());
    }

    #[test]
    fn test_sandbox_from_limits() {
        let sandbox = sandbox_from_limits(32, 500_000);
        assert_eq!(sandbox.max_memory_bytes, 32 * 1024 * 1024);
        assert_eq!(sandbox.fuel_limit, 500_000);
        assert_eq!(sandbox.timeout.as_secs(), 30); // default timeout preserved
    }

    // Integration test: loads the actual echo WASM plugin
    // Run after: cargo build --target wasm32-unknown-unknown --release -p amanclaw-skill-echo-wasm
    #[tokio::test]
    async fn test_load_and_execute_echo_wasm() {
        let wasm_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/wasm32-unknown-unknown/release/amanclaw_skill_echo_wasm.wasm");

        if !wasm_path.exists() {
            eprintln!("Skipping: echo WASM plugin not built. Run: cargo build --target wasm32-unknown-unknown --release -p amanclaw-skill-echo-wasm");
            return;
        }

        let skill = load_wasm_skill(&wasm_path, SandboxConfig::default())
            .expect("Failed to load echo WASM plugin");

        assert_eq!(skill.name, "echo");
        assert_eq!(skill.version, "0.1.0");

        let meta = Skill::metadata(&skill);
        assert_eq!(meta.name, "echo");

        let schema = skill.parameters_schema();
        assert_eq!(schema["required"][0], "text");

        let input = SkillInput {
            name: "echo".into(),
            args: r#"{"text":"Hello WASM!"}"#.into(),
            user_id: "test".into(),
            platform: "test".into(),
        };

        let result = skill.execute(input).await;
        assert!(result.success, "Execute failed: {:?}", result.error);
        assert_eq!(result.output, "Echo: Hello WASM!");
    }
}
