use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;

/// Events emitted by the dev file watcher.
#[derive(Debug)]
pub enum DevEvent {
    Plugin(String),
    Soul(String),
    Config,
}

/// Opaque guard that keeps the file watcher alive.
pub struct WatcherGuard(#[allow(dead_code)] RecommendedWatcher);

/// Watches plugins/, souls/, and config file for changes during development.
pub struct DevWatcher {
    watcher: RecommendedWatcher,
    rx: mpsc::Receiver<DevEvent>,
}

impl DevWatcher {
    pub fn new(config_path: &str) -> anyhow::Result<Self> {
        let (tx, rx) = mpsc::channel(64);
        let config_path_buf = PathBuf::from(config_path).canonicalize().ok();

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            let event = match res {
                Ok(e) => e,
                Err(_) => return,
            };

            // Only care about create/modify/remove events
            match event.kind {
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {}
                _ => return,
            }

            for path in &event.paths {
                if let Some(dev_event) = classify_path(path, config_path_buf.as_deref()) {
                    let _ = tx.blocking_send(dev_event);
                }
            }
        })?;

        // Watch plugins/ directory if it exists
        let plugins_dir = PathBuf::from("plugins");
        if plugins_dir.exists() {
            watcher.watch(&plugins_dir, RecursiveMode::Recursive)?;
        }

        // Watch souls/ directory if it exists
        let souls_dir = PathBuf::from("souls");
        if souls_dir.exists() {
            watcher.watch(&souls_dir, RecursiveMode::Recursive)?;
        }

        // Watch config file's parent directory
        let config_parent = PathBuf::from(config_path)
            .parent()
            .unwrap_or(Path::new("."))
            .to_path_buf();
        if config_parent.exists() {
            watcher.watch(&config_parent, RecursiveMode::NonRecursive)?;
        }

        Ok(Self { watcher, rx })
    }

    /// Split into a guard (keeps watcher alive) and the event receiver.
    pub fn into_parts(self) -> (WatcherGuard, mpsc::Receiver<DevEvent>) {
        (WatcherGuard(self.watcher), self.rx)
    }
}

fn classify_path(path: &Path, config_path: Option<&Path>) -> Option<DevEvent> {
    let path_str = path.to_string_lossy();

    // Check if it's the config file
    if let Some(cfg) = config_path
        && let Ok(canonical) = path.canonicalize()
        && canonical == cfg
    {
        return Some(DevEvent::Config);
    }
    // Fallback: check by filename
    let file_name = path.file_name()?.to_string_lossy();
    if file_name == "config.yaml" || file_name == "config.yml" {
        return Some(DevEvent::Config);
    }

    // Check extension for plugin files
    let ext = path.extension()?.to_string_lossy();
    match ext.as_ref() {
        "wasm" | "py" | "js" => {
            if path_str.contains("plugins") {
                return Some(DevEvent::Plugin(path_str.into_owned()));
            }
        }
        "md" => {
            if path_str.contains("souls") {
                return Some(DevEvent::Soul(path_str.into_owned()));
            }
        }
        _ => {}
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::Duration;
    use tempfile::TempDir;

    fn setup_watched_dirs(tmp: &TempDir) -> (PathBuf, PathBuf, PathBuf) {
        let plugins = tmp.path().join("plugins");
        let souls = tmp.path().join("souls");
        let config = tmp.path().join("config.yaml");
        fs::create_dir_all(&plugins).unwrap();
        fs::create_dir_all(&souls).unwrap();
        fs::write(&config, "llm:\n  model: test\n").unwrap();
        (plugins, souls, config)
    }

    #[tokio::test]
    async fn test_dev_watcher_detects_plugin() {
        let tmp = TempDir::new().unwrap();
        let (plugins, _souls, config) = setup_watched_dirs(&tmp);

        let _config_str = config.to_string_lossy().to_string();

        // Create watcher manually to watch specific dirs
        let (tx, mut rx) = mpsc::channel(64);
        let config_canon = config.canonicalize().ok();

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            let event = match res {
                Ok(e) => e,
                Err(_) => return,
            };
            match event.kind {
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {}
                _ => return,
            }
            for path in &event.paths {
                if let Some(dev_event) = classify_path(path, config_canon.as_deref()) {
                    let _ = tx.blocking_send(dev_event);
                }
            }
        })
        .unwrap();

        watcher.watch(&plugins, RecursiveMode::Recursive).unwrap();

        // Give watcher time to register
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Write a plugin file
        fs::write(plugins.join("test_plugin.py"), "print('hello')").unwrap();

        // Wait for event
        let event = tokio::time::timeout(Duration::from_secs(5), rx.recv()).await;
        assert!(event.is_ok(), "should receive an event");
        let event = event.unwrap().unwrap();
        assert!(
            matches!(event, DevEvent::Plugin(ref p) if p.contains("test_plugin.py")),
            "expected Plugin, got {event:?}",
        );

        drop(watcher);
    }

    #[tokio::test]
    async fn test_dev_watcher_detects_soul() {
        let tmp = TempDir::new().unwrap();
        let (_plugins, souls, config) = setup_watched_dirs(&tmp);

        let (tx, mut rx) = mpsc::channel(64);
        let config_canon = config.canonicalize().ok();

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            let event = match res {
                Ok(e) => e,
                Err(_) => return,
            };
            match event.kind {
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {}
                _ => return,
            }
            for path in &event.paths {
                if let Some(dev_event) = classify_path(path, config_canon.as_deref()) {
                    let _ = tx.blocking_send(dev_event);
                }
            }
        })
        .unwrap();

        watcher.watch(&souls, RecursiveMode::Recursive).unwrap();

        tokio::time::sleep(Duration::from_millis(100)).await;

        // Write a soul file
        fs::write(souls.join("default.md"), "# Soul\nYou are helpful.").unwrap();

        let event = tokio::time::timeout(Duration::from_secs(5), rx.recv()).await;
        assert!(event.is_ok(), "should receive an event");
        let event = event.unwrap().unwrap();
        assert!(
            matches!(event, DevEvent::Soul(ref p) if p.contains("default.md")),
            "expected Soul, got {event:?}",
        );

        drop(watcher);
    }

    #[test]
    fn test_classify_path_plugin() {
        let path = PathBuf::from("/some/project/plugins/my_plugin.py");
        let result = classify_path(&path, None);
        assert!(matches!(result, Some(DevEvent::Plugin(_))));
    }

    #[test]
    fn test_classify_path_soul() {
        let path = PathBuf::from("/some/project/souls/default.md");
        let result = classify_path(&path, None);
        assert!(matches!(result, Some(DevEvent::Soul(_))));
    }

    #[test]
    fn test_classify_path_config() {
        let path = PathBuf::from("/some/project/config.yaml");
        let result = classify_path(&path, None);
        assert!(matches!(result, Some(DevEvent::Config)));
    }

    #[test]
    fn test_classify_path_unrelated() {
        let path = PathBuf::from("/some/project/src/main.rs");
        let result = classify_path(&path, None);
        assert!(result.is_none());
    }
}
