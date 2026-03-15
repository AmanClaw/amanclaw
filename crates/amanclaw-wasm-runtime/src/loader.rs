use anyhow::Result;
use std::path::{Path, PathBuf};
use wasmtime::*;

/// Discovers and loads .wasm plugin files from a directory.
pub struct PluginLoader {
    engine: Engine,
    plugin_dir: PathBuf,
}

impl PluginLoader {
    pub fn new(plugin_dir: &Path) -> Result<Self> {
        let mut config = Config::new();
        config.wasm_component_model(true);
        config.async_support(true);
        config.epoch_interruption(true);

        let engine = Engine::new(&config)?;

        Ok(Self {
            engine,
            plugin_dir: plugin_dir.to_path_buf(),
        })
    }

    /// Scan plugin directory and return paths to all .wasm files.
    pub fn discover(&self) -> Result<Vec<PathBuf>> {
        let mut plugins = Vec::new();
        if !self.plugin_dir.exists() {
            tracing::warn!(dir = %self.plugin_dir.display(), "Plugin directory does not exist");
            return Ok(plugins);
        }

        for entry in std::fs::read_dir(&self.plugin_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "wasm") {
                tracing::info!(path = %path.display(), "Discovered plugin");
                plugins.push(path);
            }
        }

        Ok(plugins)
    }

    pub fn engine(&self) -> &Engine {
        &self.engine
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_discover_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let loader = PluginLoader::new(dir.path()).unwrap();
        let plugins = loader.discover().unwrap();
        assert!(plugins.is_empty());
    }

    #[test]
    fn test_discover_wasm_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("skill-a.wasm"), b"fake wasm").unwrap();
        fs::write(dir.path().join("skill-b.wasm"), b"fake wasm").unwrap();
        fs::write(dir.path().join("readme.txt"), b"not a plugin").unwrap();

        let loader = PluginLoader::new(dir.path()).unwrap();
        let plugins = loader.discover().unwrap();
        assert_eq!(plugins.len(), 2);
    }

    #[test]
    fn test_discover_nonexistent_dir() {
        let loader = PluginLoader::new(Path::new("/tmp/nonexistent-amanclaw-plugins")).unwrap();
        let plugins = loader.discover().unwrap();
        assert!(plugins.is_empty());
    }
}
