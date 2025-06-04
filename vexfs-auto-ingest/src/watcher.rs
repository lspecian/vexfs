use crate::config::Config;
use anyhow::{Context, Result};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc as tokio_mpsc;
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone)]
pub struct FileEvent {
    pub path: PathBuf,
    pub event_type: FileEventType,
    pub timestamp: Instant,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileEventType {
    Created,
    Modified,
    Deleted,
}

pub struct FileWatcher {
    config: Config,
    watcher: Option<RecommendedWatcher>,
    event_sender: tokio_mpsc::UnboundedSender<FileEvent>,
    debounce_map: HashMap<PathBuf, Instant>,
    debounce_duration: Duration,
}

impl FileWatcher {
    pub fn new(
        config: Config,
        event_sender: tokio_mpsc::UnboundedSender<FileEvent>,
    ) -> Result<Self> {
        let debounce_duration = Duration::from_millis(config.auto_ingest.debounce_ms);

        Ok(Self {
            config,
            watcher: None,
            event_sender,
            debounce_map: HashMap::new(),
            debounce_duration,
        })
    }

    pub fn start(&mut self) -> Result<()> {
        info!("Starting file watcher for paths: {:?}", self.config.auto_ingest.watch_paths);

        let (tx, rx) = mpsc::channel();
        let event_sender = self.event_sender.clone();
        let config = self.config.clone();

        // Create the watcher
        let mut watcher = RecommendedWatcher::new(
            move |res: notify::Result<Event>| {
                if let Err(e) = tx.send(res) {
                    error!("Failed to send file system event: {}", e);
                }
            },
            notify::Config::default(),
        )
        .context("Failed to create file watcher")?;

        // Watch all configured paths
        for path in &self.config.auto_ingest.watch_paths {
            if path.exists() {
                watcher
                    .watch(path, RecursiveMode::Recursive)
                    .with_context(|| format!("Failed to watch path: {}", path.display()))?;
                info!("Watching path: {}", path.display());
            } else {
                warn!("Watch path does not exist: {}", path.display());
            }
        }

        self.watcher = Some(watcher);

        // Spawn event processing task
        tokio::spawn(async move {
            Self::process_events(rx, event_sender, config).await;
        });

        info!("File watcher started successfully");
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(watcher) = self.watcher.take() {
            drop(watcher);
            info!("File watcher stopped");
        }
    }

    async fn process_events(
        rx: mpsc::Receiver<notify::Result<Event>>,
        event_sender: tokio_mpsc::UnboundedSender<FileEvent>,
        config: Config,
    ) {
        let mut debounce_map: HashMap<PathBuf, Instant> = HashMap::new();
        let debounce_duration = Duration::from_millis(config.auto_ingest.debounce_ms);

        // Process events in a blocking task to avoid blocking the async runtime
        tokio::task::spawn_blocking(move || {
            for res in rx {
                match res {
                    Ok(event) => {
                        if let Err(e) = Self::handle_event(
                            event,
                            &event_sender,
                            &config,
                            &mut debounce_map,
                            debounce_duration,
                        ) {
                            error!("Error handling file event: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("File watcher error: {}", e);
                    }
                }
            }
        })
        .await
        .unwrap_or_else(|e| {
            error!("File watcher task panicked: {}", e);
        });
    }

    fn handle_event(
        event: Event,
        event_sender: &tokio_mpsc::UnboundedSender<FileEvent>,
        config: &Config,
        debounce_map: &mut HashMap<PathBuf, Instant>,
        debounce_duration: Duration,
    ) -> Result<()> {
        let now = Instant::now();

        for path in event.paths {
            // Skip if not a file
            if !path.is_file() {
                continue;
            }

            // Check if file should be processed
            if !config.should_process_file(&path) {
                debug!("Skipping file (not in include patterns): {}", path.display());
                continue;
            }

            // Debounce events for the same file
            if let Some(&last_event_time) = debounce_map.get(&path) {
                if now.duration_since(last_event_time) < debounce_duration {
                    debug!("Debouncing event for file: {}", path.display());
                    continue;
                }
            }

            debounce_map.insert(path.clone(), now);

            // Determine event type
            let event_type = match event.kind {
                EventKind::Create(_) => FileEventType::Created,
                EventKind::Modify(_) => FileEventType::Modified,
                EventKind::Remove(_) => FileEventType::Deleted,
                _ => {
                    debug!("Ignoring event type: {:?} for file: {}", event.kind, path.display());
                    continue;
                }
            };

            // Skip delete events for now (we don't need to process deleted files)
            if event_type == FileEventType::Deleted {
                debug!("Skipping delete event for file: {}", path.display());
                continue;
            }

            let file_event = FileEvent {
                path: path.clone(),
                event_type,
                timestamp: now,
            };

            debug!("Sending file event: {:?}", file_event);

            if let Err(e) = event_sender.send(file_event) {
                error!("Failed to send file event: {}", e);
            }
        }

        // Clean up old debounce entries
        let cutoff = now - debounce_duration * 10; // Keep entries for 10x debounce duration
        debounce_map.retain(|_, &mut timestamp| timestamp > cutoff);

        Ok(())
    }

    /// Add a new path to watch
    pub fn add_watch_path<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path = path.as_ref();
        
        if let Some(ref mut watcher) = self.watcher {
            watcher
                .watch(path, RecursiveMode::Recursive)
                .with_context(|| format!("Failed to add watch path: {}", path.display()))?;
            
            info!("Added watch path: {}", path.display());
        }

        Ok(())
    }

    /// Remove a path from watching
    pub fn remove_watch_path<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path = path.as_ref();
        
        if let Some(ref mut watcher) = self.watcher {
            watcher
                .unwatch(path)
                .with_context(|| format!("Failed to remove watch path: {}", path.display()))?;
            
            info!("Removed watch path: {}", path.display());
        }

        Ok(())
    }

    /// Get current debounce statistics
    pub fn get_debounce_stats(&self) -> DebounceStats {
        DebounceStats {
            active_files: self.debounce_map.len(),
            debounce_duration_ms: self.debounce_duration.as_millis() as u64,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DebounceStats {
    pub active_files: usize,
    pub debounce_duration_ms: u64,
}

impl Drop for FileWatcher {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Utility function to check if a path matches any of the given patterns
pub fn matches_patterns(path: &Path, patterns: &[String]) -> bool {
    let file_name = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    for pattern in patterns {
        if glob_match(pattern, file_name) {
            return true;
        }
    }

    false
}

/// Simple glob pattern matching
fn glob_match(pattern: &str, text: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    
    if pattern.starts_with("*.") {
        let ext = &pattern[2..];
        return text.ends_with(ext);
    }
    
    if pattern.ends_with("/*") {
        let prefix = &pattern[..pattern.len() - 2];
        return text.starts_with(prefix);
    }
    
    pattern == text
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::sync::mpsc;

    #[test]
    fn test_glob_match() {
        assert!(glob_match("*.txt", "file.txt"));
        assert!(glob_match("*.md", "README.md"));
        assert!(!glob_match("*.txt", "file.md"));
        assert!(glob_match("*", "anything"));
    }

    #[test]
    fn test_matches_patterns() {
        let patterns = vec!["*.txt".to_string(), "*.md".to_string()];
        
        assert!(matches_patterns(Path::new("test.txt"), &patterns));
        assert!(matches_patterns(Path::new("README.md"), &patterns));
        assert!(!matches_patterns(Path::new("test.rs"), &patterns));
    }

    #[tokio::test]
    async fn test_file_watcher_creation() {
        let config = Config::default();
        let (tx, _rx) = mpsc::unbounded_channel();
        
        let watcher = FileWatcher::new(config, tx);
        assert!(watcher.is_ok());
    }

    #[tokio::test]
    async fn test_file_event_creation() {
        let event = FileEvent {
            path: PathBuf::from("test.txt"),
            event_type: FileEventType::Created,
            timestamp: Instant::now(),
        };

        assert_eq!(event.event_type, FileEventType::Created);
        assert_eq!(event.path, PathBuf::from("test.txt"));
    }

    #[tokio::test]
    async fn test_debounce_stats() {
        let config = Config::default();
        let (tx, _rx) = mpsc::unbounded_channel();
        
        let watcher = FileWatcher::new(config, tx).unwrap();
        let stats = watcher.get_debounce_stats();
        
        assert_eq!(stats.active_files, 0);
        assert_eq!(stats.debounce_duration_ms, 1000); // Default from config
    }
}