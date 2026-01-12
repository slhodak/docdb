You are building a minimal document database in Rust for learning purposes. The goal is correctness, clarity, and testability, not performance.

Design constraints:
Single-node, embedded database. Append-only log for storage. In-memory primary-key index. JSON documents stored as bytes. API supports put(key, document), get(key), delete(key). Database must have on-disk persistence - all data must be written to disk and survive process restarts. Database must recover correctly after crashes.

Requirements:
Write idiomatic Rust. Keep modules small and explicit. No unsafe code. No async. No external crates except serde_json for JSON and tempfile for tests. Every public function must have a test.

Crash safety:
All writes go through an append-only log. On startup, the database must replay the log to rebuild the in-memory index. Deletions must be represented explicitly.

Testing:
Write deterministic tests that simulate crash/restart by reopening the database directory. Add a property-style test: after any sequence of operations and restart, get returns the last written value or None if deleted.

Deliverables:
Start by implementing only:
A Log struct that appends records to disk.
A Db struct that wraps the log and an in-memory HashMap index.
put, get, delete, open, close.

Explain invariants in comments. Prefer simple, explicit code over cleverness. Do not add features beyond this scope.

Begin by designing the on-disk record format and implementing the Log layer with tests.

Write a CLI for this rust-based document DB.

Write a README.md for this project that includes the instructions for building and running it locally in the terminal.