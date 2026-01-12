# DocDB

A simple, crash-safe document database written in Rust with a command-line interface.

## Features

- **Crash-safe persistence**: All writes are logged to an append-only log before updating the in-memory index
- **JSON document storage**: Store and retrieve JSON documents by key
- **Simple CLI**: Easy-to-use command-line interface for all operations
- **Recovery**: Automatically recovers database state from the log on startup

## Building

### Prerequisites

- Rust (1.70 or later recommended)
- Cargo (comes with Rust)

### Build from Source

1. Clone the repository:
   ```bash
   git clone <repository-url>
   cd docdb
   ```

2. Build the release binary:
   ```bash
   cargo build --release
   ```

   The binary will be located at `target/release/docdb`.

## Installation

### Option 1: Install Globally (Recommended)

Install the binary to your Cargo bin directory (usually `~/.cargo/bin`):

```bash
cargo install --path .
```

This will make `docdb` available in your terminal if `~/.cargo/bin` is in your PATH (which it should be by default with Rust installations).

### Option 2: Manual Installation

1. Build the release binary (see above)

2. Copy the binary to a directory in your PATH:
   ```bash
   cp target/release/docdb /usr/local/bin/
   ```
   
   Or create a symlink:
   ```bash
   ln -s $(pwd)/target/release/docdb /usr/local/bin/docdb
   ```

## Running Locally

After building or installing, you can run `docdb` directly from the terminal:

```bash
# Show help
docdb --help

# Or run the binary directly from the build directory
./target/release/docdb --help
```

## Usage

### Basic Commands

**Store a document:**
```bash
docdb put user1 '{"name": "Alice", "age": 30, "email": "alice@example.com"}'
```

**Store from stdin:**
```bash
echo '{"name": "Bob", "age": 25}' | docdb put user2
```

**Retrieve a document:**
```bash
docdb get user1
```

**List all keys:**
```bash
docdb list
```

**Delete a document:**
```bash
docdb delete user1
```

### Database Directory

By default, the database is stored in the current directory. You can specify a custom directory:

```bash
docdb --db-dir /path/to/database put key1 '{"value": "test"}'
docdb --db-dir /path/to/database get key1
```

The database creates a `log` file in the specified directory to store all operations.

## Examples

### Example Workflow

```bash
# Create a new database in the current directory
docdb put product1 '{"name": "Laptop", "price": 999.99, "stock": 42}'
docdb put product2 '{"name": "Mouse", "price": 29.99, "stock": 100}'

# List all products
docdb list
# Output:
# product1
# product2

# Retrieve a product
docdb get product1
# Output:
# {
#   "name": "Laptop",
#   "price": 999.99,
#   "stock": 42
# }

# Update a product
docdb put product1 '{"name": "Laptop", "price": 899.99, "stock": 40}'

# Delete a product
docdb delete product2
```

### Using a Custom Database Directory

```bash
# Create a database in a specific directory
mkdir -p ~/my-database
docdb --db-dir ~/my-database put config '{"theme": "dark", "language": "en"}'
docdb --db-dir ~/my-database get config
```

## Project Structure

```
docdb/
├── Cargo.toml          # Project dependencies and metadata
├── src/
│   ├── main.rs         # CLI entry point
│   ├── db.rs           # Database implementation
│   └── log.rs          # Append-only log implementation
└── README.md           # This file
```

## How It Works

- **Storage**: Documents are stored as JSON strings, validated before being written
- **Persistence**: All operations (put/delete) are written to an append-only log file
- **Recovery**: On startup, the database replays the log to rebuild the in-memory index
- **Index**: An in-memory HashMap provides fast key lookups

## Development

### Running Tests

```bash
cargo test
```

### Building for Development

```bash
cargo build
```

The debug binary will be at `target/debug/docdb`.

## License

[Add your license here]
