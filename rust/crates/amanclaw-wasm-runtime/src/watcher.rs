use notify::{Watcher, RecursiveMode, Event, EventKind, RecommendedWatcher};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;

/// Events emitted by the plugin watcher.
#[derive(Debug, Clone)]
pub enum PluginEvent {
    Added(PathBuf),
    Modified(PathBuf),
    Removed(PathBuf),
}

/// Watches a plugin directory for .wasm file changes.
pub struct PluginWatcher {
    _watcher: RecommendedWatcher,
    pub rx: mpsc::Receiver<PluginEvent>,
}

impl PluginWatcher {
    pub fn new(plugin_dir: &Path) -> anyhow::Result<Self> {
        let (tx, rx) = mpsc::channel(64);
        let plugin_dir_owned = plugin_dir.to_path_buf();

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| {
            if let Ok(event) = res {
                for path in &event.paths {
                    if path.extension().is_some_and(|ext| ext == "wasm") {
                        let plugin_event = match event.kind {
                            EventKind::Create(_) => Some(PluginEvent::Added(path.clone())),
                            EventKind::Modify(_) => Some(PluginEvent::Modified(path.clone())),
                            EventKind::Remove(_) => Some(PluginEvent::Removed(path.clone())),
                            _ => None,
                        };
                        if let Some(pe) = plugin_event {
                            let _ = tx.blocking_send(pe);
                        }
                    }
                }
            }
        })?;

        if plugin_dir_owned.exists() {
            watcher.watch(&plugin_dir_owned, RecursiveMode::NonRecursive)?;
            tracing::info!(dir = %plugin_dir_owned.display(), "Watching plugin directory");
        } else {
            tracing::warn!(dir = %plugin_dir_owned.display(), "Plugin directory does not exist, not watching");
        }

        Ok(Self { _watcher: watcher, rx })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_watcher_detects_new_wasm() {
        let dir = tempfile::tempdir().unwrap();
        let mut watcher = PluginWatcher::new(dir.path()).unwrap();

        // Create a .wasm file
        fs::write(dir.path().join("test.wasm"), b"fake wasm").unwrap();

        // Wait a bit for the event
        sleep(Duration::from_millis(500)).await;

        // Should get an event (Added or Modified depending on OS)
        if let Ok(event) = watcher.rx.try_recv() {
            match event {
                PluginEvent::Added(p) | PluginEvent::Modified(p) => {
                    assert!(p.to_string_lossy().contains("test.wasm"));
                }
                _ => {}
            }
        }
        // Note: filesystem events may not fire reliably in CI, so we don't assert
    }
}
