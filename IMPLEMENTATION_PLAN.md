# Implementation Plan: Document Database (DocDB)

This document provides a prioritized task list for implementing a minimal, crash-safe document database in Rust.

## Role in Workflow

**This is a living document that tracks implementation progress across multiple iterations.**

### How This Plan Is Used

1. **Task Selection**: During BUILDING mode, the model reads this plan and selects the most important uncompleted task to work on.

2. **Incremental Updates**: After completing a task, the model **updates** this plan (does NOT overwrite it):
   - Marks completed tasks as done (e.g., `[x]` or `✅`)
   - Adds notes about discoveries, bugs, or implementation details
   - Updates task status or priorities if needed
   - Documents any deviations from the original plan

3. **Progress Tracking**: The plan serves as a persistent record of:
   - What has been completed
   - What remains to be done
   - Important learnings discovered during implementation
   - Any issues or blockers encountered

### Important Rules

- **NEVER overwrite this file** - Always update it incrementally
- **Preserve completed work** - Mark tasks done, don't delete them
- **Document discoveries** - Add notes about implementation details, edge cases, or design decisions
- **Keep it current** - Update the plan after each task completion to reflect the current state

### When to Regenerate

This plan should only be completely regenerated during **PLANNING mode** when:
- No plan exists yet, OR
- The plan is stale or incorrect (specs have changed significantly, or code has diverged from plan)

During normal BUILDING mode iterations, always update incrementally.

## Overview

DocDB is a single-node, embedded document database that stores JSON documents by key. It uses an append-only log for crash-safe persistence and an in-memory HashMap for fast lookups. The design prioritizes correctness, clarity, and testability over performance.

### Critical Requirement: On-Disk Persistence

**All data must be written to disk and survive process restarts.**

- **All writes must be persisted**: Every `put()` and `delete()` operation must write to disk before completing
- **Data survives restarts**: After a process restart, all previously written data must be recoverable
- **No data loss**: The in-memory index is ephemeral; the log file is the source of truth
- **Recovery on startup**: On every `Db::open()`, the database must replay the log to rebuild the in-memory index from disk

## Prioritized Task List

### Phase 1: Log Layer (`log.rs`) - Foundation

**Priority: HIGHEST** - All other components depend on this.

- [x] **Task 1.1**: Define record types and constants
  - Define `RECORD_PUT = 0x00` and `RECORD_DELETE = 0x01` constants
  - Create `LogRecord` enum with `Put { key, value }` and `Delete { key }` variants
  - Status: ✅ Complete

- [x] **Task 1.2**: Implement `Log` struct with `open()` method
  - Create struct with `BufWriter<File>` field
  - Implement `open(path)` that opens/creates file in append mode
  - Use `OpenOptions::new().create(true).append(true)`
  - Status: ✅ Complete

- [x] **Task 1.3**: Implement `Log::put()` method
  - Write record type byte (0x00)
  - Write key length as u32 little-endian
  - Write key bytes
  - Write value length as u32 little-endian
  - Write value bytes
  - **Flush** to ensure data is on disk
  - Status: ✅ Complete

- [x] **Task 1.4**: Implement `Log::delete()` method
  - Write record type byte (0x01)
  - Write key length as u32 little-endian
  - Write key bytes
  - **Flush** to ensure data is on disk
  - Status: ✅ Complete

- [x] **Task 1.5**: Implement `Log::read_all()` method
  - Open file for reading
  - Loop until EOF:
    - Read record type byte
    - Read key length (4 bytes)
    - Read key (key_length bytes)
    - If Put: read value length and value
    - Construct LogRecord and add to vector
  - Handle EOF gracefully (not an error)
  - Return error on unknown record type
  - Status: ✅ Complete

- [x] **Task 1.6**: Write comprehensive tests for log layer
  - Single Put record write and read
  - Single Delete record write and read
  - Multiple records in sequence
  - Empty keys and values
  - Large keys and values (stress test)
  - Reopen and append (verify append mode works)
  - Status: ✅ Complete

### Phase 2: Database Layer (`db.rs`) - Core Logic

**Priority: HIGH** - Implements the main database operations.

- [x] **Task 2.1**: Define `Db` struct
  - Fields: `log_path: PathBuf`, `log: Log`, `index: HashMap<String, Vec<u8>>`
  - Document invariants in comments
  - Status: ✅ Complete

- [x] **Task 2.2**: Implement `replay_log()` helper function
  - Create empty HashMap
  - If log file doesn't exist, return empty HashMap
  - Read all records from log using `Log::read_all()`
  - For each record:
    - If `Put { key, value }`: Convert key to String (skip if invalid UTF-8), insert into HashMap
    - If `Delete { key }`: Convert key to String (skip if invalid UTF-8), remove from HashMap
  - Return the HashMap
  - Status: ✅ Complete

- [x] **Task 2.3**: Implement `Db::open()` method
  - Construct log path: `dir.join("log")`
  - Create directory if it doesn't exist
  - **Replay log to rebuild index from disk** - This ensures all persisted data is recovered
  - Open log for appending
  - Return Db instance with index reflecting all data that survived from previous runs
  - Status: ✅ Complete

- [x] **Task 2.4**: Implement `Db::put()` method
  - **Write to log first** (ensures on-disk persistence and crash safety)
  - **Flush log to disk** (ensures data survives process restart)
  - Update in-memory index
  - Return Ok(()) - only after data is persisted to disk
  - Status: ✅ Complete

- [x] **Task 2.5**: Implement `Db::get()` method
  - Look up key in index
  - Return `Option<&[u8]>` (None if not found)
  - Status: ✅ Complete

- [x] **Task 2.6**: Implement `Db::delete()` method
  - **Write to log first** (ensures on-disk persistence and crash safety)
  - **Flush log to disk** (ensures deletion survives process restart)
  - Remove from in-memory index
  - Return Ok(()) - only after deletion is persisted to disk
  - Status: ✅ Complete

- [x] **Task 2.7**: Implement `Db::keys()` method
  - Return iterator over all keys: `self.index.keys()`
  - Status: ✅ Complete

- [x] **Task 2.8**: Implement `Db::close()` method
  - Currently a no-op (log is dropped, which flushes buffers)
  - Return Ok(())
  - Status: ✅ Complete

- [x] **Task 2.9**: Write comprehensive tests for database layer
  - Basic put and get
  - Delete operation
  - Put overwrite (same key, different value)
  - Multiple keys
  - **Recovery after put (close and reopen)** - Verify data persists across restarts
  - **Recovery after delete (close and reopen)** - Verify deletions persist across restarts
  - **Recovery after sequence of operations** - Verify complex state persists
  - Empty database
  - Automatic directory creation
  - **Property-style test: after any sequence of operations and restart, get returns the last written value or None if deleted** - This verifies on-disk persistence works correctly
  - Status: ✅ Complete

### Phase 3: CLI Layer (`main.rs`) - User Interface

**Priority: MEDIUM** - Provides user-facing interface.

- [x] **Task 3.1**: Define CLI structure with clap
  - Use `clap` with derive feature
  - Define `Cli` struct with `db_dir: PathBuf` (default: ".")
  - Define `Commands` enum: `Put`, `Get`, `Delete`, `List`, `Info`
  - Status: ✅ Complete

- [x] **Task 3.2**: Implement `handle_put()` function
  - If value provided: validate JSON, convert to bytes
  - If value not provided: read from stdin, validate JSON, convert to bytes
  - Open database
  - Call `db.put(key, value_bytes)`
  - Close database
  - Handle errors gracefully
  - Status: ✅ Complete

- [x] **Task 3.3**: Implement `handle_get()` function
  - Open database
  - Call `db.get(key)`
  - If found: try to parse as JSON and pretty-print, fallback to string output
  - If not found: print error and exit with code 1
  - Handle errors gracefully
  - Status: ✅ Complete

- [x] **Task 3.4**: Implement `handle_delete()` function
  - Open database
  - Call `db.delete(key)`
  - Close database
  - Handle errors gracefully
  - Status: ✅ Complete

- [x] **Task 3.5**: Implement `handle_list()` function
  - Open database
  - Collect all keys, sort them
  - Print each key (or "No keys found" if empty)
  - Status: ✅ Complete

- [x] **Task 3.6**: Implement `handle_info()` function
  - Resolve absolute path of database directory
  - Display database directory path
  - Display log file path (db_dir.join("log"))
  - Optionally show log file size if it exists
  - Status: ✅ Complete

- [x] **Task 3.7**: Implement JSON validation
  - Use `serde_json::from_str::<serde_json::Value>()` to validate JSON
  - Reject invalid JSON with clear error messages
  - For get operations, try to pretty-print JSON, fallback to raw string
  - Status: ✅ Complete

### Phase 4: Verification and Polish

**Priority: LOW** - Final checks and documentation.

- [ ] **Task 4.1**: Verify all requirements are met
  - [ ] All writes go through the log before updating the index
  - [ ] All data is written to disk
  - [ ] Database recovers correctly after crashes
  - [ ] Data survives process restarts
  - [ ] All public functions have tests
  - [ ] Tests pass with `cargo test`
  - [ ] CLI works correctly for all operations
  - [ ] JSON validation works (accepts valid JSON, rejects invalid)
  - [ ] Property-style recovery test passes
  - [ ] Code compiles without warnings
  - [ ] No unsafe code is used
  - [ ] Documentation (comments) explains invariants

- [ ] **Task 4.2**: Update README.md with user documentation
  - Installation instructions
  - Usage examples for each command
  - Database location and file structure
  - Status: Pending

## Reference: Architecture

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

## Reference: On-Disk Record Format

The log file uses a binary format for efficiency and crash safety. Each record is written atomically.

### Record Structure

#### Put Record
```
┌─────────────┬──────────────┬─────────┬──────────────┬─────────┐
│ Record Type │ Key Length   │ Key     │ Value Length │ Value   │
│ (1 byte)   │ (4 bytes)    │ (N)     │ (4 bytes)    │ (M)     │
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

## Reference: On-Disk Persistence and Crash Safety

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

## Reference: Edge Cases

1. **Empty log file**: Return empty index
2. **Corrupted log file**: Return error on unknown record type
3. **Invalid UTF-8 keys**: Skip during replay (or return error)
4. **Empty keys/values**: Supported (length = 0)
5. **Very large keys/values**: Should work (tested with 10KB keys, 50KB values)
6. **Concurrent access**: Not supported (single-node, embedded design)
7. **Missing database directory**: Created automatically on first write

## Reference: Dependencies

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

## Reference: File Structure

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

## Out of Scope

The following features are explicitly out of scope for the initial implementation:
- Log compaction (log file grows indefinitely)
- Transactions
- Multi-threading or async operations
- Network access
- Query capabilities beyond key lookup
- Indexes beyond the primary key index
