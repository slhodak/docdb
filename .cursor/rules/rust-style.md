# Rust Style Guide

## General Principles
- Follow Rust conventions and best practices
- Use idiomatic Rust patterns
- Prefer safe Rust - no `unsafe` code blocks

## Type System
- Use `PathBuf` and `AsRef<Path>` for flexible path handling
- Join paths using `join()` method
- Check for file existence before reading when appropriate

## I/O Operations
- Use `BufWriter` for efficient writes
- Always flush after critical operations to ensure data reaches disk
- The buffer is automatically flushed when dropped
- Use `OpenOptions` for file operations with explicit flags

## Encoding
- All multi-byte integers (u32 for lengths) are written in little-endian format
- Use `to_le_bytes()` when writing
- Use `u32::from_le_bytes()` when reading

## Error Handling
- Prefer `Result` types over panicking
- Use `std::io::Result` for I/O operations
- Handle EOF gracefully (not an error in read loops)
- Return errors on unknown/corrupted data

## Module Organization
- Keep each module focused and explicit
- Small modules are preferred
- Document public APIs with doc comments
- Use `#[cfg(test)]` for test modules
