# Coding Guidelines

## Project Goals
- Correctness, clarity, and testability are the primary goals
- Prefer explicit code over cleverness

## Code Requirements
- Write idiomatic Rust
- Keep modules small and explicit
- No unsafe code
- Minimize external dependencies - only use what is necessary
- Every public function must have a test
- Clear comments: Explain invariants and design decisions

## Error Handling
- Use `std::io::Result` for I/O operations
- Return errors rather than panicking
- Provide clear error messages in CLI
- Handle edge cases gracefully (e.g., invalid UTF-8 keys)

## API Design
- Public functions should be testable
- Keep the API simple and focused
- Document invariants in comments
- Prefer explicit code over cleverness

## Testing Requirements
- Write deterministic tests
- Include property-style tests where appropriate
- Tests should verify correctness after state changes and restarts
- Use `tempfile` crate for temporary directories/files in tests
- Create databases in temporary directories
- Clean up automatically after tests
- Test edge cases: empty values, large values, corrupted data, etc.
