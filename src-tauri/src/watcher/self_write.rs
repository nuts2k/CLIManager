use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;

/// Tracks files written by this app to avoid re-processing our own writes
/// as incoming sync events. Entries expire after 1 second.
pub struct SelfWriteTracker {
    writes: Mutex<HashMap<PathBuf, Instant>>,
}

impl SelfWriteTracker {
    pub fn new() -> Self {
        Self {
            writes: Mutex::new(HashMap::new()),
        }
    }

    /// Record that we just wrote to the given path.
    pub fn record_write(&self, path: PathBuf) {
        let mut map = self.writes.lock().unwrap();
        map.insert(path, Instant::now());
    }

    /// Check if the given path was written by us within the last 1 second.
    /// Also cleans up expired entries.
    pub fn is_self_write(&self, path: &PathBuf) -> bool {
        let mut map = self.writes.lock().unwrap();
        let now = Instant::now();
        let threshold = std::time::Duration::from_secs(1);

        // Clean up expired entries
        map.retain(|_, timestamp| now.duration_since(*timestamp) < threshold);

        // Check if path is still in the map (i.e., written within 1 second)
        map.contains_key(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_record_and_detect_self_write() {
        let tracker = SelfWriteTracker::new();
        let path = PathBuf::from("/tmp/test-provider.json");

        tracker.record_write(path.clone());
        assert!(tracker.is_self_write(&path));
    }

    #[test]
    fn test_unrecorded_path_is_not_self_write() {
        let tracker = SelfWriteTracker::new();
        let path = PathBuf::from("/tmp/unknown.json");

        assert!(!tracker.is_self_write(&path));
    }

    #[test]
    fn test_expired_write_is_not_self_write() {
        let tracker = SelfWriteTracker::new();
        let path = PathBuf::from("/tmp/expired.json");

        tracker.record_write(path.clone());
        // Wait for expiry (1 second + margin)
        thread::sleep(Duration::from_millis(1100));
        assert!(!tracker.is_self_write(&path));
    }

    #[test]
    fn test_cleanup_removes_expired_entries() {
        let tracker = SelfWriteTracker::new();
        let old_path = PathBuf::from("/tmp/old.json");
        let new_path = PathBuf::from("/tmp/new.json");

        tracker.record_write(old_path.clone());
        thread::sleep(Duration::from_millis(1100));
        tracker.record_write(new_path.clone());

        // Checking new_path should also clean up old_path
        assert!(tracker.is_self_write(&new_path));
        assert!(!tracker.is_self_write(&old_path));
    }

    #[test]
    fn test_recent_write_within_window() {
        let tracker = SelfWriteTracker::new();
        let path = PathBuf::from("/tmp/recent.json");

        tracker.record_write(path.clone());
        thread::sleep(Duration::from_millis(500));
        // Still within 1 second window
        assert!(tracker.is_self_write(&path));
    }
}
