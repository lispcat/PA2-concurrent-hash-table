//! # Commands Module
//!
//! Parses the `commands.txt` file into a list of typed `Command` values.
//!
//! ## Input Format
//! The first line is: `threads,<num_threads>,<unused>`
//! Subsequent lines follow these formats:
//!   - `insert,<name>,<salary>,<priority>`
//!   - `delete,<name>,<unused>,<priority>`
//!   - `update,<name>,<new_salary>,<priority>`
//!   - `search,<name>,<unused>,<priority>`
//!   - `print,<unused>,<unused>,<priority>`
//!
//! All commands have PRIORITY as the LAST field.

use std::fs;

/// Parsed representation of a single command from commands.txt.
///
/// ## C vs Rust: Enums
/// In C you'd use an integer constant (`#define INSERT 1`) and a union.
/// Rust enums are "algebraic data types": each variant carries its own data,
/// and `match` forces you to handle every case at compile time.
#[derive(Debug, Clone)]
pub enum Command {
    Insert {
        name: String,
        salary: u32,
        priority: u32,
    },
    Delete {
        name: String,
        priority: u32,
    },
    Update {
        name: String,
        salary: u32,
        priority: u32,
    },
    Search {
        name: String,
        priority: u32,
    },
    Print {
        priority: u32,
    },
}

impl Command {
    /// Get the priority (thread ordering index) for this command.
    pub fn priority(&self) -> u32 {
        match self {
            Command::Insert { priority, .. } => *priority,
            Command::Delete { priority, .. } => *priority,
            Command::Update { priority, .. } => *priority,
            Command::Search { priority, .. } => *priority,
            Command::Print { priority } => *priority,
        }
    }
}

/// Parse commands.txt, returning the thread count and sorted list of commands.
///
/// All commands use PRIORITY as the LAST comma-separated field.
/// The fields between the command verb and priority vary by command type.
pub fn parse_commands(path: &str) -> (u32, Vec<Command>) {
    let content = fs::read_to_string(path).expect("Failed to read commands.txt");
    let mut lines = content.lines();
    let mut num_threads = 0u32;
    let mut commands = Vec::new();

    // First line: `threads,<count>,<unused>`
    if let Some(first_line) = lines.next() {
        let parts: Vec<&str> = first_line.trim().splitn(4, ',').collect();
        if parts.len() >= 2 && parts[0].trim().to_lowercase() == "threads" {
            num_threads = parts[1].trim().parse().unwrap_or(0);
        }
    }

    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Split into parts. Priority is ALWAYS the LAST element.
        let parts: Vec<&str> = line.split(',').collect();
        if parts.is_empty() {
            continue;
        }

        let verb = parts[0].trim().to_lowercase();

        // Helper: get the last field as priority (u32)
        let last_priority = || -> u32 {
            parts
                .last()
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0)
        };

        let cmd = match verb.as_str() {
            // insert,<name>,<salary>,<priority>
            "insert" if parts.len() >= 4 => {
                let name = parts[1].trim().to_string();
                // salary is second-to-last before priority, but for insert it's parts[2]
                // since format is: insert, name, salary, priority (4 fields)
                let salary: u32 = parts[parts.len() - 2].trim().parse().unwrap_or(0);
                let priority = last_priority();
                Some(Command::Insert {
                    name,
                    salary,
                    priority,
                })
            }
            // delete,<name>,<unused>,<priority>  (4 fields)
            // or delete,<name>,<priority>        (3 fields)
            "delete" if parts.len() >= 3 => {
                let name = parts[1].trim().to_string();
                let priority = last_priority();
                Some(Command::Delete { name, priority })
            }
            // update,<name>,<new_salary>,<priority>
            "update" if parts.len() >= 4 => {
                let name = parts[1].trim().to_string();
                let salary: u32 = parts[parts.len() - 2].trim().parse().unwrap_or(0);
                let priority = last_priority();
                Some(Command::Update {
                    name,
                    salary,
                    priority,
                })
            }
            // search,<name>,<unused>,<priority>  (4 fields)
            // or search,<name>,<priority>        (3 fields)
            "search" if parts.len() >= 3 => {
                let name = parts[1].trim().to_string();
                let priority = last_priority();
                Some(Command::Search { name, priority })
            }
            // print,<unused>,<unused>,<priority>
            "print" if parts.len() >= 2 => {
                let priority = last_priority();
                Some(Command::Print { priority })
            }
            _ => {
                eprintln!("Warning: unrecognized command line: {}", line);
                None
            }
        };

        if let Some(cmd) = cmd {
            commands.push(cmd);
        }
    }

    // Sort by priority so thread_idx == priority for our turn-ordering.
    // In C: qsort() with a comparator function pointer.
    // In Rust: sort_by_key() with a closure (anonymous function).
    commands.sort_by_key(|c| c.priority());

    (num_threads, commands)
}
