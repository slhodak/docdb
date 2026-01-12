use crate::log::{Log, LogRecord};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// In-memory document database with crash-safe persistence.
/// 
/// Invariants:
/// - All writes go through the append-only log before updating the index.
/// - The index always reflects the state after replaying all log records.
/// - Keys are stored as strings (for JSON compatibility).
/// - Values are stored as raw bytes (JSON documents as bytes).
pub struct Db {
    /// Path to the log file for persistence.
    log_path: PathBuf,
    /// Append-only log for crash-safe writes.
    log: Log,
    /// In-memory index mapping keys to values.
    /// 
    /// Invariant: A key is present in the index if and only if it has been
    /// put and not deleted (or deleted then put again).
    index: HashMap<String, Vec<u8>>,
}

impl Db {
    /// Opens or creates a database at the given directory.
    /// 
    /// On startup, replays the log to rebuild the in-memory index.
    /// This ensures crash recovery: the database state matches what it was
    /// before the crash.
    pub fn open<P: AsRef<Path>>(dir: P) -> std::io::Result<Self> {
        let dir = dir.as_ref();
        let log_path = dir.join("log");
        
        // Replay the log to rebuild the index
        let index = Self::replay_log(&log_path)?;
        
        // Open the log for appending new records
        let log = Log::open(&log_path)?;
        
        Ok(Db {
            log_path,
            log,
            index,
        })
    }

    /// Replays the log file to rebuild the in-memory index.
    /// 
    /// Invariant: After replay, the index contains the state that results
    /// from applying all log records in order. Later operations overwrite
    /// earlier ones (Put overwrites previous Put/Delete, Delete removes the key).
    fn replay_log<P: AsRef<Path>>(log_path: P) -> std::io::Result<HashMap<String, Vec<u8>>> {
        let mut index = HashMap::new();
        
        // If the log file doesn't exist yet, return an empty index
        if !log_path.as_ref().exists() {
            return Ok(index);
        }
        
        // Read all records from the log
        let records = Log::read_all(log_path)?;
        
        // Apply each record to rebuild the index
        for record in records {
            match record {
                LogRecord::Put { key, value } => {
                    // Convert key from bytes to string
                    // If the key is not valid UTF-8, we skip it (could also return an error)
                    if let Ok(key_str) = String::from_utf8(key) {
                        index.insert(key_str, value);
                    }
                }
                LogRecord::Delete { key } => {
                    // Convert key from bytes to string and remove from index
                    if let Ok(key_str) = String::from_utf8(key) {
                        index.remove(&key_str);
                    }
                }
            }
        }
        
        Ok(index)
    }

    /// Stores a key-value pair in the database.
    /// 
    /// The value is stored as raw bytes (JSON documents should be serialized
    /// to bytes before calling this method).
    /// 
    /// Invariant: The operation is logged before the index is updated,
    /// ensuring crash safety.
    pub fn put(&mut self, key: &str, value: &[u8]) -> std::io::Result<()> {
        // Write to log first (crash safety)
        self.log.put(key.as_bytes(), value)?;
        
        // Update in-memory index
        self.index.insert(key.to_string(), value.to_vec());
        
        Ok(())
    }

    /// Retrieves a value by key.
    /// 
    /// Returns None if the key doesn't exist or was deleted.
    pub fn get(&self, key: &str) -> Option<&[u8]> {
        self.index.get(key).map(|v| v.as_slice())
    }

    /// Deletes a key from the database.
    /// 
    /// Invariant: The deletion is logged before the index is updated,
    /// ensuring crash safety.
    pub fn delete(&mut self, key: &str) -> std::io::Result<()> {
        // Write to log first (crash safety)
        self.log.delete(key.as_bytes())?;
        
        // Update in-memory index
        self.index.remove(key);
        
        Ok(())
    }

    /// Returns an iterator over all keys in the database.
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.index.keys()
    }

    /// Closes the database.
    /// 
    /// Currently a no-op, but provided for API completeness.
    /// The log file is automatically flushed on each write.
    pub fn close(self) -> std::io::Result<()> {
        // Log is dropped here, which will flush any remaining buffers
        // No explicit action needed
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_put_and_get() {
        let temp_dir = TempDir::new().unwrap();
        let mut db = Db::open(temp_dir.path()).unwrap();
        
        db.put("key1", b"value1").unwrap();
        
        assert_eq!(db.get("key1"), Some(b"value1".as_slice()));
        assert_eq!(db.get("nonexistent"), None);
    }

    #[test]
    fn test_delete() {
        let temp_dir = TempDir::new().unwrap();
        let mut db = Db::open(temp_dir.path()).unwrap();
        
        db.put("key1", b"value1").unwrap();
        assert_eq!(db.get("key1"), Some(b"value1".as_slice()));
        
        db.delete("key1").unwrap();
        assert_eq!(db.get("key1"), None);
    }

    #[test]
    fn test_put_overwrite() {
        let temp_dir = TempDir::new().unwrap();
        let mut db = Db::open(temp_dir.path()).unwrap();
        
        db.put("key1", b"value1").unwrap();
        db.put("key1", b"value2").unwrap();
        
        assert_eq!(db.get("key1"), Some(b"value2".as_slice()));
    }

    #[test]
    fn test_multiple_keys() {
        let temp_dir = TempDir::new().unwrap();
        let mut db = Db::open(temp_dir.path()).unwrap();
        
        db.put("key1", b"value1").unwrap();
        db.put("key2", b"value2").unwrap();
        db.put("key3", b"value3").unwrap();
        
        assert_eq!(db.get("key1"), Some(b"value1".as_slice()));
        assert_eq!(db.get("key2"), Some(b"value2".as_slice()));
        assert_eq!(db.get("key3"), Some(b"value3".as_slice()));
    }

    #[test]
    fn test_recovery_after_put() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create database, put a value, and close
        {
            let mut db = Db::open(temp_dir.path()).unwrap();
            db.put("key1", b"value1").unwrap();
            db.close().unwrap();
        }
        
        // Reopen and verify the value is still there
        let db = Db::open(temp_dir.path()).unwrap();
        assert_eq!(db.get("key1"), Some(b"value1".as_slice()));
    }

    #[test]
    fn test_recovery_after_delete() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create database, put and delete, then close
        {
            let mut db = Db::open(temp_dir.path()).unwrap();
            db.put("key1", b"value1").unwrap();
            db.delete("key1").unwrap();
            db.close().unwrap();
        }
        
        // Reopen and verify the key is gone
        let db = Db::open(temp_dir.path()).unwrap();
        assert_eq!(db.get("key1"), None);
    }

    #[test]
    fn test_recovery_after_sequence() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create database, perform sequence of operations, then close
        {
            let mut db = Db::open(temp_dir.path()).unwrap();
            db.put("key1", b"value1").unwrap();
            db.put("key2", b"value2").unwrap();
            db.put("key1", b"value1_updated").unwrap();
            db.delete("key2").unwrap();
            db.put("key3", b"value3").unwrap();
            db.close().unwrap();
        }
        
        // Reopen and verify final state
        let db = Db::open(temp_dir.path()).unwrap();
        assert_eq!(db.get("key1"), Some(b"value1_updated".as_slice()));
        assert_eq!(db.get("key2"), None);
        assert_eq!(db.get("key3"), Some(b"value3".as_slice()));
    }

    #[test]
    fn test_empty_database() {
        let temp_dir = TempDir::new().unwrap();
        let db = Db::open(temp_dir.path()).unwrap();
        
        assert_eq!(db.get("anykey"), None);
    }

    #[test]
    fn test_property_style_recovery() {
        // Property-style test: after any sequence of operations and restart,
        // get returns the last written value or None if deleted.
        let temp_dir = TempDir::new().unwrap();
        
        // Test multiple sequences: (operations, expected_final_state)
        // expected_final_state is a map of key -> expected value (None if deleted)
        let test_cases: Vec<(Vec<(&str, &str, Option<&str>)>, Vec<(&str, Option<&str>)>)> = vec![
            (
                vec![("put", "key1", Some("value1"))],
                vec![("key1", Some("value1"))],
            ),
            (
                vec![("put", "key1", Some("value1")), ("delete", "key1", None)],
                vec![("key1", None)],
            ),
            (
                vec![
                    ("put", "key1", Some("value1")),
                    ("put", "key1", Some("value2")),
                ],
                vec![("key1", Some("value2"))],
            ),
            (
                vec![
                    ("put", "key1", Some("value1")),
                    ("put", "key2", Some("value2")),
                    ("delete", "key1", None),
                    ("put", "key1", Some("value3")),
                ],
                vec![("key1", Some("value3")), ("key2", Some("value2"))],
            ),
        ];
        
        for (seq_num, (operations, expected_final_state)) in test_cases.iter().enumerate() {
            // Clear the database
            let _ = std::fs::remove_file(temp_dir.path().join("log"));
            
            // Apply sequence of operations
            {
                let mut db = Db::open(temp_dir.path()).unwrap();
                for (op, key, value) in operations {
                    match *op {
                        "put" => {
                            db.put(key, value.unwrap().as_bytes()).unwrap();
                        }
                        "delete" => {
                            db.delete(key).unwrap();
                        }
                        _ => panic!("Unknown operation: {}", op),
                    }
                }
                db.close().unwrap();
            }
            
            // Reopen and verify final state matches expectations
            let db = Db::open(temp_dir.path()).unwrap();
            for (key, expected_value) in expected_final_state {
                let actual = db.get(key);
                let expected = expected_value.map(|v| v.as_bytes());
                assert_eq!(
                    actual,
                    expected,
                    "Sequence {}: key '{}' should be {:?} after restart",
                    seq_num,
                    key,
                    expected_value
                );
            }
        }
    }
}
