mod log;
mod db;

use clap::{Parser, Subcommand};
use db::Db;
use std::io::{self, Read};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "docdb")]
#[command(about = "A CLI for the document database", long_about = None)]
struct Cli {
    /// Database directory path (defaults to current directory)
    #[arg(long, default_value = ".")]
    db_dir: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Store a key-value pair in the database
    Put {
        /// The key to store
        key: String,
        /// The JSON value to store (if not provided, reads from stdin)
        value: Option<String>,
    },
    /// Retrieve a value by key
    Get {
        /// The key to retrieve
        key: String,
    },
    /// Delete a key from the database
    Delete {
        /// The key to delete
        key: String,
    },
    /// List all keys in the database
    List,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Put { key, value } => {
            handle_put(&cli.db_dir, &key, value);
        }
        Commands::Get { key } => {
            handle_get(&cli.db_dir, &key);
        }
        Commands::Delete { key } => {
            handle_delete(&cli.db_dir, &key);
        }
        Commands::List => {
            handle_list(&cli.db_dir);
        }
    }
}

fn handle_put(db_dir: &PathBuf, key: &str, value: Option<String>) {
    let value_bytes = match value {
        Some(v) => {
            // Validate that it's valid JSON
            match serde_json::from_str::<serde_json::Value>(&v) {
                Ok(_) => v.into_bytes(),
                Err(e) => {
                    eprintln!("Error: Invalid JSON: {}", e);
                    std::process::exit(1);
                }
            }
        }
        None => {
            // Read from stdin
            let mut buffer = String::new();
            io::stdin()
                .read_to_string(&mut buffer)
                .expect("Failed to read from stdin");
            
            // Validate JSON
            match serde_json::from_str::<serde_json::Value>(&buffer) {
                Ok(_) => buffer.into_bytes(),
                Err(e) => {
                    eprintln!("Error: Invalid JSON from stdin: {}", e);
                    std::process::exit(1);
                }
            }
        }
    };

    let mut db = match Db::open(db_dir) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Error: Failed to open database: {}", e);
            std::process::exit(1);
        }
    };

    match db.put(key, &value_bytes) {
        Ok(()) => {
            // Success - no output for put operations
        }
        Err(e) => {
            eprintln!("Error: Failed to put value: {}", e);
            std::process::exit(1);
        }
    }

    if let Err(e) = db.close() {
        eprintln!("Warning: Failed to close database: {}", e);
    }
}

fn handle_get(db_dir: &PathBuf, key: &str) {
    let db = match Db::open(db_dir) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Error: Failed to open database: {}", e);
            std::process::exit(1);
        }
    };

    match db.get(key) {
        Some(value_bytes) => {
            // Try to parse as JSON and pretty-print
            match serde_json::from_slice::<serde_json::Value>(value_bytes) {
                Ok(json_value) => {
                    match serde_json::to_string_pretty(&json_value) {
                        Ok(pretty) => println!("{}", pretty),
                        Err(e) => {
                            eprintln!("Error: Failed to format JSON: {}", e);
                            // Fall back to raw output
                            match String::from_utf8(value_bytes.to_vec()) {
                                Ok(s) => println!("{}", s),
                                Err(_) => {
                                    eprintln!("Error: Value is not valid UTF-8 or JSON");
                                    std::process::exit(1);
                                }
                            }
                        }
                    }
                }
                Err(_) => {
                    // Not valid JSON, try to output as string
                    match String::from_utf8(value_bytes.to_vec()) {
                        Ok(s) => println!("{}", s),
                        Err(_) => {
                            eprintln!("Error: Value is not valid UTF-8");
                            std::process::exit(1);
                        }
                    }
                }
            }
        }
        None => {
            eprintln!("Error: Key '{}' not found", key);
            std::process::exit(1);
        }
    }
}

fn handle_delete(db_dir: &PathBuf, key: &str) {
    let mut db = match Db::open(db_dir) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Error: Failed to open database: {}", e);
            std::process::exit(1);
        }
    };

    match db.delete(key) {
        Ok(()) => {
            // Success - no output for delete operations
        }
        Err(e) => {
            eprintln!("Error: Failed to delete key: {}", e);
            std::process::exit(1);
        }
    }

    if let Err(e) = db.close() {
        eprintln!("Warning: Failed to close database: {}", e);
    }
}

fn handle_list(db_dir: &PathBuf) {
    let db = match Db::open(db_dir) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Error: Failed to open database: {}", e);
            std::process::exit(1);
        }
    };

    let mut keys: Vec<&String> = db.keys().collect();
    keys.sort();

    if keys.is_empty() {
        println!("No keys found in database");
    } else {
        for key in keys {
            println!("{}", key);
        }
    }
}
