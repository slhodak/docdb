use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Read, Write};
use std::path::Path;

/// Record type identifiers for the append-only log.
/// 
/// Invariant: Each record type has a unique byte value.
const RECORD_PUT: u8 = 0;
const RECORD_DELETE: u8 = 1;

/// Represents a single operation in the log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogRecord {
    /// Put operation: store a key-value pair.
    Put { key: Vec<u8>, value: Vec<u8> },
    /// Delete operation: remove a key.
    Delete { key: Vec<u8> },
}

/// Append-only log for crash-safe persistence.
/// 
/// Invariants:
/// - All writes are appended to the end of the file.
/// - Records are never modified or deleted from the log.
/// - The log file is opened in append mode to prevent accidental overwrites.
/// 
/// Record format (binary):
/// - Record type: 1 byte (0 = Put, 1 = Delete)
/// - Key length: 4 bytes (u32, little-endian)
/// - Key: N bytes (where N = key length)
/// - For Put records only:
///   - Value length: 4 bytes (u32, little-endian)
///   - Value: M bytes (where M = value length)
pub struct Log {
    writer: BufWriter<File>,
}

impl Log {
    /// Opens or creates a log file at the given path.
    /// 
    /// The file is opened in append mode to ensure all writes go to the end.
    /// If the file doesn't exist, it will be created.
    pub fn open<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        Ok(Log {
            writer: BufWriter::new(file),
        })
    }

    /// Appends a Put record to the log.
    /// 
    /// Invariant: The record is written atomically (all bytes are written
    /// before returning, or an error is returned).
    pub fn put(&mut self, key: &[u8], value: &[u8]) -> std::io::Result<()> {
        // Write record type
        self.writer.write_all(&[RECORD_PUT])?;
        
        // Write key length and key
        let key_len = key.len() as u32;
        self.writer.write_all(&key_len.to_le_bytes())?;
        self.writer.write_all(key)?;
        
        // Write value length and value
        let value_len = value.len() as u32;
        self.writer.write_all(&value_len.to_le_bytes())?;
        self.writer.write_all(value)?;
        
        // Flush to ensure data is written to disk
        self.writer.flush()?;
        
        Ok(())
    }

    /// Appends a Delete record to the log.
    /// 
    /// Invariant: The record is written atomically (all bytes are written
    /// before returning, or an error is returned).
    pub fn delete(&mut self, key: &[u8]) -> std::io::Result<()> {
        // Write record type
        self.writer.write_all(&[RECORD_DELETE])?;
        
        // Write key length and key
        let key_len = key.len() as u32;
        self.writer.write_all(&key_len.to_le_bytes())?;
        self.writer.write_all(key)?;
        
        // Flush to ensure data is written to disk
        self.writer.flush()?;
        
        Ok(())
    }

    /// Reads all records from a log file.
    /// 
    /// This is used during recovery to rebuild the in-memory index.
    /// Returns an error if the log file is corrupted or unreadable.
    pub fn read_all<P: AsRef<Path>>(path: P) -> std::io::Result<Vec<LogRecord>> {
        let mut file = File::open(path)?;
        let mut records = Vec::new();
        
        loop {
            // Try to read record type
            let mut record_type_buf = [0u8; 1];
            match file.read_exact(&mut record_type_buf) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    // End of file reached, this is normal
                    break;
                }
                Err(e) => return Err(e),
            }
            
            let record_type = record_type_buf[0];
            
            // Read key length
            let mut key_len_buf = [0u8; 4];
            file.read_exact(&mut key_len_buf)?;
            let key_len = u32::from_le_bytes(key_len_buf) as usize;
            
            // Read key
            let mut key = vec![0u8; key_len];
            file.read_exact(&mut key)?;
            
            match record_type {
                RECORD_PUT => {
                    // Read value length
                    let mut value_len_buf = [0u8; 4];
                    file.read_exact(&mut value_len_buf)?;
                    let value_len = u32::from_le_bytes(value_len_buf) as usize;
                    
                    // Read value
                    let mut value = vec![0u8; value_len];
                    file.read_exact(&mut value)?;
                    
                    records.push(LogRecord::Put { key, value });
                }
                RECORD_DELETE => {
                    records.push(LogRecord::Delete { key });
                }
                _ => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("Unknown record type: {}", record_type),
                    ));
                }
            }
        }
        
        Ok(records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_put_record() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        
        let mut log = Log::open(path).unwrap();
        log.put(b"key1", b"value1").unwrap();
        
        let records = Log::read_all(path).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(
            records[0],
            LogRecord::Put {
                key: b"key1".to_vec(),
                value: b"value1".to_vec()
            }
        );
    }

    #[test]
    fn test_delete_record() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        
        let mut log = Log::open(path).unwrap();
        log.delete(b"key1").unwrap();
        
        let records = Log::read_all(path).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(
            records[0],
            LogRecord::Delete {
                key: b"key1".to_vec()
            }
        );
    }

    #[test]
    fn test_multiple_records() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        
        let mut log = Log::open(path).unwrap();
        log.put(b"key1", b"value1").unwrap();
        log.put(b"key2", b"value2").unwrap();
        log.delete(b"key1").unwrap();
        log.put(b"key3", b"value3").unwrap();
        
        let records = Log::read_all(path).unwrap();
        assert_eq!(records.len(), 4);
        assert_eq!(
            records[0],
            LogRecord::Put {
                key: b"key1".to_vec(),
                value: b"value1".to_vec()
            }
        );
        assert_eq!(
            records[1],
            LogRecord::Put {
                key: b"key2".to_vec(),
                value: b"value2".to_vec()
            }
        );
        assert_eq!(
            records[2],
            LogRecord::Delete {
                key: b"key1".to_vec()
            }
        );
        assert_eq!(
            records[3],
            LogRecord::Put {
                key: b"key3".to_vec(),
                value: b"value3".to_vec()
            }
        );
    }

    #[test]
    fn test_empty_key_and_value() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        
        let mut log = Log::open(path).unwrap();
        log.put(b"", b"").unwrap();
        log.put(b"key", b"").unwrap();
        log.put(b"", b"value").unwrap();
        
        let records = Log::read_all(path).unwrap();
        assert_eq!(records.len(), 3);
        assert_eq!(
            records[0],
            LogRecord::Put {
                key: vec![],
                value: vec![]
            }
        );
        assert_eq!(
            records[1],
            LogRecord::Put {
                key: b"key".to_vec(),
                value: vec![]
            }
        );
        assert_eq!(
            records[2],
            LogRecord::Put {
                key: vec![],
                value: b"value".to_vec()
            }
        );
    }

    #[test]
    fn test_large_key_and_value() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        
        let large_key = vec![0u8; 10000];
        let large_value = vec![1u8; 50000];
        
        let mut log = Log::open(path).unwrap();
        log.put(&large_key, &large_value).unwrap();
        
        let records = Log::read_all(path).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].key().len(), 10000);
        assert_eq!(records[0].value().unwrap().len(), 50000);
    }

    #[test]
    fn test_reopen_and_append() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        
        {
            let mut log = Log::open(path).unwrap();
            log.put(b"key1", b"value1").unwrap();
        }
        
        {
            let mut log = Log::open(path).unwrap();
            log.put(b"key2", b"value2").unwrap();
        }
        
        let records = Log::read_all(path).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(
            records[0],
            LogRecord::Put {
                key: b"key1".to_vec(),
                value: b"value1".to_vec()
            }
        );
        assert_eq!(
            records[1],
            LogRecord::Put {
                key: b"key2".to_vec(),
                value: b"value2".to_vec()
            }
        );
    }
}

// Helper methods for tests
impl LogRecord {
    fn key(&self) -> &[u8] {
        match self {
            LogRecord::Put { key, .. } => key,
            LogRecord::Delete { key } => key,
        }
    }

    fn value(&self) -> Option<&[u8]> {
        match self {
            LogRecord::Put { value, .. } => Some(value),
            LogRecord::Delete { .. } => None,
        }
    }
}
