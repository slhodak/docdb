# Implementation Plan: Document Database (DocDB)

This document provides a comprehensive guide for implementing a minimal, crash-safe document database in Rust. It is based on the analysis of the existing implementation and the original requirements.

## Overview

DocDB is a single-node, embedded document database that stores JSON documents by key. It uses an append-only log for crash-safe persistence and an in-memory HashMap for fast lookups. The design prioritizes correctness, clarity, and testability over performance.

### On-Disk Persistence Requirement

**Critical Requirement:** The database must have on-disk persistence. All data must be written to disk and survive process restarts. This means:

- **All writes must be persisted**: Every `put()` and `delete()` operation must write to disk before completing
- **Data survives restarts**: After a process restart, all previously written data must be recoverable
- **No data loss**: The in-memory index is ephemeral; the log file is the source of truth
- **Recovery on startup**: On every `Db::open()`, the database must replay the log to rebuild the in-memory index from disk

## Architecture

### High-Level Design

```
┌─────────────────────────────────────────┐
│              CLI (main.rs)              │
│  - Command parsing (clap)               │
│  - JSON validation                       │
│  - User interaction                      │
└──────────────┬──────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────┐
│          Database (db.rs)                │
│  - In-memory index (HashMap)             │
│  - put/get/delete operations             │
│  - Log replay on startup                 │
└──────────────┬──────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────┐
│         Log Layer (log.rs)               │
│  - Append-only file operations           │
│  - Binary record format                  │
│  - Crash-safe writes                     │
└──────────────┬──────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────┐
│         File System                      │
│  - log file (append-only)                │
└─────────────────────────────────────────┘
```

### Key Components

1. **Log Layer** (`log.rs`): Handles all disk I/O with an append-only log file
2. **Database Layer** (`db.rs`): Manages the in-memory index and coordinates with the log
3. **CLI Layer** (`main.rs`): Provides user interface and JSON validation

## On-Disk Record Format

The log file uses a binary format for efficiency and crash safety. Each record is written atomically.

### Record Structure

#### Put Record
```
┌─────────────┬──────────────┬─────────┬──────────────┬─────────┐
│ Record Type │ Key Length   │ Key     │ Value Length │ Value   │
│ (1 byte)    │ (4 bytes)    │ (N)     │ (4 bytes)    │ (M)     │
│ 0x00        │ u32 LE       │ bytes   │ u32 LE       │ bytes   │
└─────────────┴──────────────┴─────────┴──────────────┴─────────┘
```

#### Delete Record
```
┌─────────────┬──────────────┬─────────┐
│ Record Type │ Key Length   │ Key     │
│ (1 byte)    │ (4 bytes)    │ (N)     │
│ 0x01        │ u32 LE       │ bytes   │
└─────────────┴──────────────┴─────────┘
```

### Record Type Constants
- `RECORD_PUT = 0x00`: Indicates a Put operation
- `RECORD_DELETE = 0x01`: Indicates a Delete operation

### Format Details

1. **Record Type**: Single byte identifying the operation type
2. **Key Length**: 4-byte unsigned integer in little-endian format
3. **Key**: Variable-length byte array (length specified by Key Length)
4. **Value Length** (Put only): 4-byte unsigned integer in little-endian format
5. **Value** (Put only): Variable-length byte array (length specified by Value Length)

### Invariants

- All writes are appended to the end of the file (never overwrite)
- Each record is written atomically (all bytes written or none)
- The log file is opened in append mode to prevent accidental overwrites
- After each write, the buffer is flushed to ensure data reaches disk

## Implementation Steps

### Phase 1: Log Layer (`log.rs`)

#### Step 1.1: Define Record Types

```rust
// Record type constants
const RECORD_PUT: u8 = 0;
const RECORD_DELETE: u8 = 1;

// LogRecord enum
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogRecord {
    Put { key: Vec<u8>, value: Vec<u8> },
    Delete { key: Vec<u8> },
}
```

#### Step 1.2: Implement Log Struct

```rust
pub struct Log {
    writer: BufWriter<File>,
}
```

**Key Methods:**

1. **`open(path)`**: Opens or creates log file in append mode
   - Use `OpenOptions::new().create(true).append(true)`
   - Wrap file in `BufWriter` for efficiency

2. **`put(key, value)`**: Writes a Put record
   - Write record type byte (0x00)
   - Write key length as u32 little-endian
   - Write key bytes
   - Write value length as u32 little-endian
   - Write value bytes
   - **Flush** to ensure data is on disk

3. **`delete(key)`**: Writes a Delete record
   - Write record type byte (0x01)
   - Write key length as u32 little-endian
   - Write key bytes
   - **Flush** to ensure data is on disk

4. **`read_all(path)`**: Reads all records from log file
   - Open file for reading
   - Loop until EOF:
     - Read record type byte
     - Read key length (4 bytes)
     - Read key (key_length bytes)
     - If Put: read value length and value
     - Construct LogRecord and add to vector
   - Handle EOF gracefully (not an error)
   - Return error on unknown record type

#### Step 1.3: Write Tests for Log Layer

Test cases to implement:
- Single Put record write and read
- Single Delete record write and read
- Multiple records in sequence
- Empty keys and values
- Large keys and values (stress test)
- Reopen and append (verify append mode works)

### Phase 2: Database Layer (`db.rs`)

#### Step 2.1: Define Db Struct

```rust
pub struct Db {
    log_path: PathBuf,
    log: Log,
    index: HashMap<String, Vec<u8>>,
}
```

**Invariants:**
- **All writes go through the log before updating the index** - This ensures on-disk persistence
- **The index always reflects the state after replaying all log records** - The log is the source of truth
- **Data survives restarts** - The index is rebuilt from the log on every `open()`
- Keys are stored as strings (for JSON compatibility)
- Values are stored as raw bytes (JSON documents as bytes)

#### Step 2.2: Implement Recovery (Log Replay)

```rust
fn replay_log(log_path: P) -> std::io::Result<HashMap<String, Vec<u8>>>
```

**Algorithm:**
1. Create empty HashMap
2. If log file doesn't exist, return empty HashMap
3. Read all records from log using `Log::read_all()`
4. For each record:
   - If `Put { key, value }`: Convert key to String (skip if invalid UTF-8), insert into HashMap
   - If `Delete { key }`: Convert key to String (skip if invalid UTF-8), remove from HashMap
5. Return the HashMap

**Note:** Later operations overwrite earlier ones (Put overwrites previous Put/Delete, Delete removes the key).

#### Step 2.3: Implement Database Operations

1. **`open(dir)`**: Opens or creates database
   - Construct log path: `dir.join("log")`
   - **Replay log to rebuild index from disk** - This ensures all persisted data is recovered
   - Open log for appending
   - Return Db instance with index reflecting all data that survived from previous runs

2. **`put(key, value)`**: Stores a key-value pair
   - **Write to log first** (ensures on-disk persistence and crash safety)
   - **Flush log to disk** (ensures data survives process restart)
   - Update in-memory index
   - Return Ok(()) - only after data is persisted to disk

3. **`get(key)`**: Retrieves a value by key
   - Look up key in index
   - Return `Option<&[u8]>` (None if not found)

4. **`delete(key)`**: Deletes a key
   - **Write to log first** (ensures on-disk persistence and crash safety)
   - **Flush log to disk** (ensures deletion survives process restart)
   - Remove from in-memory index
   - Return Ok(()) - only after deletion is persisted to disk

5. **`keys()`**: Returns iterator over all keys
   - Return `self.index.keys()`

6. **`close()`**: Closes the database
   - Currently a no-op (log is dropped, which flushes buffers)
   - Return Ok(())

#### Step 2.4: Write Tests for Database Layer

Test cases to implement:
- Basic put and get
- Delete operation
- Put overwrite (same key, different value)
- Multiple keys
- **Recovery after put (close and reopen)** - Verify data persists across restarts
- **Recovery after delete (close and reopen)** - Verify deletions persist across restarts
- **Recovery after sequence of operations** - Verify complex state persists
- Empty database
- **Property-style test: after any sequence of operations and restart, get returns the last written value or None if deleted** - This verifies on-disk persistence works correctly

### Phase 3: CLI Layer (`main.rs`)

#### Step 3.1: Define CLI Structure

Use `clap` with derive feature:

```rust
#[derive(Parser)]
struct Cli {
    #[arg(long, default_value = ".")]
    db_dir: PathBuf,
    
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Put { key: String, value: Option<String> },
    Get { key: String },
    Delete { key: String },
    List,
    Info,
}
```

#### Step 3.2: Implement Command Handlers

1. **`handle_put(db_dir, key, value)`**:
   - If value provided: validate JSON, convert to bytes
   - If value not provided: read from stdin, validate JSON, convert to bytes
   - Open database
   - Call `db.put(key, value_bytes)`
   - Close database
   - Handle errors gracefully

2. **`handle_get(db_dir, key)`**:
   - Open database
   - Call `db.get(key)`
   - If found: try to parse as JSON and pretty-print, fallback to string output
   - If not found: print error and exit with code 1
   - Handle errors gracefully

3. **`handle_delete(db_dir, key)`**:
   - Open database
   - Call `db.delete(key)`
   - Close database
   - Handle errors gracefully

4. **`handle_list(db_dir)`**:
   - Open database
   - Collect all keys, sort them
   - Print each key (or "No keys found" if empty)

5. **`handle_info(db_dir)`**:
   - Resolve absolute path of database directory
   - Display database directory path
   - Display log file path (db_dir.join("log"))
   - Optionally show log file size if it exists

#### Step 3.3: JSON Validation

- Use `serde_json::from_str::<serde_json::Value>()` to validate JSON
- Reject invalid JSON with clear error messages
- For get operations, try to pretty-print JSON, fallback to raw string

## On-Disk Persistence and Crash Safety

### Persistence Guarantees

**Fundamental Requirement:** All data must be written to disk and survive process restarts.

- **Complete persistence**: Every `put()` and `delete()` operation writes to the log file on disk
- **Survives restarts**: All data written before a process restart is recoverable after restart
- **Log is source of truth**: The in-memory index is rebuilt from the log on every startup
- **No in-memory-only data**: The index is never the only copy of data; it is always derived from the log

### Write Ordering

**Critical Invariant:** All writes to the log must complete before updating the in-memory index.

This ensures that if a crash occurs:
- The log contains the operation
- On recovery, the operation will be replayed
- The database state will be consistent
- **No data loss**: All operations that completed before the crash are preserved on disk

### Recovery Process

**On every `Db::open()`:**

1. The log file is read from disk (if it exists)
2. All log records are read in order
3. Each record is applied to rebuild the in-memory index
4. The final index state matches what it was before the restart
5. **All data survives**: Any data written before the restart is fully recoverable

This recovery process ensures that:
- Process restarts do not lose data
- The database state is always consistent with what was written to disk
- The in-memory index accurately reflects the persisted log

### Flush Strategy

- After each `put()` or `delete()` operation, the log buffer is flushed
- This ensures data reaches disk before the operation completes
- Prevents data loss if the process crashes immediately after an operation
- **Guarantees persistence**: Once an operation returns successfully, the data is guaranteed to be on disk

## Testing Strategy

### Unit Tests

Each module should have comprehensive unit tests:
- **log.rs**: Test record format, read/write operations, edge cases
- **db.rs**: Test all operations, recovery scenarios, edge cases

### Integration Tests

- Test full workflows through the CLI
- **Test on-disk persistence** - Verify data survives process restarts
- Test crash recovery by simulating restarts (close and reopen database)
- Test property-style recovery (any sequence of operations should recover correctly)
- **Verify all data is on disk** - Test that closing and reopening preserves all data

### Test Utilities

- Use `tempfile` crate for temporary directories/files
- Create databases in temporary directories
- Clean up automatically after tests

### Property-Style Test

Implement a test that:
1. Generates random sequences of put/delete operations
2. Applies them to a database
3. Closes and reopens the database
4. Verifies that `get()` returns the correct final state for each key

## Dependencies

### Required Dependencies

```toml
[dependencies]
serde_json = "1.0"  # JSON parsing and validation
clap = { version = "4.5", features = ["derive"] }  # CLI parsing
```

### Dev Dependencies

```toml
[dev-dependencies]
tempfile = "3.8"  # Temporary files for testing
```

## Design Principles

### Code Quality

- **Idiomatic Rust**: Follow Rust conventions and best practices
- **No unsafe code**: Use only safe Rust
- **No async**: Keep it simple, synchronous I/O only
- **Small modules**: Keep each module focused and explicit
- **Clear comments**: Explain invariants and design decisions

### Error Handling

- Use `std::io::Result` for I/O operations
- Return errors rather than panicking
- Provide clear error messages in CLI
- Handle edge cases gracefully (e.g., invalid UTF-8 keys)

### API Design

- Public functions should be testable
- Keep the API simple and focused
- Document invariants in comments
- Prefer explicit code over cleverness

## File Structure

```
docdb/
├── Cargo.toml          # Project configuration and dependencies
├── src/
│   ├── main.rs         # CLI entry point and command handlers
│   ├── db.rs           # Database implementation (index + log coordination)
│   └── log.rs          # Append-only log implementation
├── README.md           # User documentation
└── IMPLEMENTATION_PLAN.md  # This file
```

## Implementation Order

1. **Start with log.rs**: Implement the storage layer first
   - Define record format
   - Implement write operations
   - Implement read operations
   - Write comprehensive tests

2. **Then db.rs**: Build the database layer on top of the log
   - Implement recovery (log replay)
   - Implement put/get/delete
   - Write comprehensive tests including recovery scenarios

3. **Finally main.rs**: Add the CLI interface
   - Define CLI structure
   - Implement command handlers
   - Add JSON validation
   - Test end-to-end workflows

## Key Implementation Details

### Little-Endian Encoding

All multi-byte integers (u32 for lengths) are written in little-endian format:
- Use `to_le_bytes()` when writing
- Use `u32::from_le_bytes()` when reading

### UTF-8 Key Handling

- Keys are stored as bytes in the log
- Keys are converted to String when building the index
- Invalid UTF-8 keys are skipped during replay (could also return an error)

### Buffer Management

- Use `BufWriter` for efficient writes
- Always flush after critical operations
- The buffer is automatically flushed when dropped

### Path Handling

- Use `PathBuf` and `AsRef<Path>` for flexible path handling
- Join paths using `join()` method
- Check for file existence before reading

## Edge Cases to Handle

1. **Empty log file**: Return empty index
2. **Corrupted log file**: Return error on unknown record type
3. **Invalid UTF-8 keys**: Skip during replay (or return error)
4. **Empty keys/values**: Supported (length = 0)
5. **Very large keys/values**: Should work (tested with 10KB keys, 50KB values)
6. **Concurrent access**: Not supported (single-node, embedded design)
7. **Missing database directory**: Created automatically on first write

## Future Enhancements (Out of Scope)

The following features are explicitly out of scope for the initial implementation:
- Log compaction (log file grows indefinitely)
- Transactions
- Multi-threading or async operations
- Network access
- Query capabilities beyond key lookup
- Indexes beyond the primary key index

## Verification Checklist

After implementation, verify:

- [ ] **All writes go through the log before updating the index** - Ensures on-disk persistence
- [ ] **All data is written to disk** - Every put/delete operation persists to log file
- [ ] **Database recovers correctly after crashes** - Test by closing and reopening
- [ ] **Data survives process restarts** - All previously written data is recoverable
- [ ] All public functions have tests
- [ ] Tests pass with `cargo test`
- [ ] CLI works correctly for all operations
- [ ] JSON validation works (accepts valid JSON, rejects invalid)
- [ ] Property-style recovery test passes (verifies persistence)
- [ ] Code compiles without warnings
- [ ] No unsafe code is used
- [ ] Documentation (comments) explains invariants

## Conclusion

This implementation plan provides a complete guide for building a crash-safe document database. The key to success is:

1. **Start with the log layer** - get the storage format right
2. **Implement recovery correctly** - this is critical for crash safety
3. **Test thoroughly** - especially recovery scenarios
4. **Keep it simple** - prefer clarity over cleverness

The design prioritizes correctness and testability, making it an excellent learning project for understanding database internals, crash safety, and Rust best practices.
