//! # Concurrent Hash Table — Main Entry Point
//!
//! This program demonstrates a concurrent hash table using reader-writer locks
//! and condition variables for ordered thread execution.
//!
//! ## Architecture
//! - `hash.rs`     — HashRecord struct and Jenkins hash function
//! - `table.rs`    — Insert/delete/update/search/print operations on the table
//! - `logger.rs`   — Thread-safe timestamped logging to hash.log
//! - `rwlock.rs`   — Logged RwLock wrapper
//! - `commands.rs` — Parse commands.txt into typed Command values
//! - `main.rs`     — Thread spawning, synchronization, and dispatch

mod commands;
mod hash;
mod logger;
mod rwlock;
mod table;

use std::sync::{Arc, Condvar, Mutex};
use std::thread;

use commands::{parse_commands, Command};
use hash::{jenkins_hash, HashRecord};
use logger::Logger;
use rwlock::LoggedRwLock;
use table::{table_delete, table_insert, table_print, table_search, table_update};

type TurnState = Arc<(Mutex<u32>, Condvar)>;

fn main() {
    let (_num_threads, commands) = parse_commands("commands.txt");

    let logger = Logger::new("hash.log");

    let table: Arc<LoggedRwLock<Vec<HashRecord>>> =
        Arc::new(LoggedRwLock::new(Vec::new(), Arc::clone(&logger)));

    let turn_state: TurnState = Arc::new((Mutex::new(0u32), Condvar::new()));

    let mut handles: Vec<thread::JoinHandle<()>> = Vec::new();

    for (thread_idx, command) in commands.into_iter().enumerate() {
        let thread_id = thread_idx as u32;
        let priority = command.priority();

        let table_clone  = Arc::clone(&table);
        let logger_clone = Arc::clone(&logger);
        let turn_clone   = Arc::clone(&turn_state);

        let handle = thread::spawn(move || {
            let (lock, cvar) = &*turn_clone;

            // Phase 1: wait for our turn
            logger_clone.log_waiting(priority);
            {
                let mut current_turn = lock.lock().unwrap();
                while *current_turn != thread_id {
                    current_turn = cvar.wait(current_turn).unwrap();
                }
            }
            logger_clone.log_awakened(priority);

            // Phase 2: signal next thread before doing our work
            {
                let mut current_turn = lock.lock().unwrap();
                *current_turn += 1;
                cvar.notify_all();
            }

            // Phase 3: perform the operation under the RwLock
            execute_command(&command, &table_clone, &logger_clone, priority);
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("a worker thread panicked");
    }

    // Final print — after all threads complete.
    // Use priority 0 for the final lock log entries, matching the spec's
    // convention of not assigning a special thread id to the post-join print.
    {
        // Log the lock acquisition/release counts before the final print.
        let (acquisitions, releases) = logger.lock_counts();
        logger.log_counts(acquisitions, releases);

        let records = table.read_lock(0);
        let output  = table_print(&records);
        table.log_read_released(0);
        drop(records);

        // Write "Final Table:" and all records to the log as well.
        logger.log_final_table(&output);

        print!("{}", output);
    }
}

/// Execute one command against the hash table.
fn execute_command(
    command: &Command,
    table: &Arc<LoggedRwLock<Vec<HashRecord>>>,
    logger: &Arc<Logger>,
    priority: u32,
) {
    match command {
        Command::Insert { name, salary, .. } => {
            // Log format: INSERT,<hash>,<name>,<salary>
            let hash_val = jenkins_hash(name);
            logger.log_command(priority, &format!("INSERT,{},{},{}", hash_val, name, salary));

            let mut records = table.write_lock(priority);
            let result = table_insert(&mut records, name, *salary);
            table.log_write_released(priority);
            drop(records);

            match result {
                Ok(record) => println!("Inserted {}", record),
                Err(h)     => println!("Insert failed. Entry {} is a duplicate.", h),
            }
        }

        Command::Delete { name, .. } => {
            // Log format: DELETE,<hash>,<name>
            let hash_val = jenkins_hash(name);
            logger.log_command(priority, &format!("DELETE,{},{}", hash_val, name));

            let mut records = table.write_lock(priority);
            let result = table_delete(&mut records, name);
            table.log_write_released(priority);
            drop(records);

            match result {
                Some(record) => println!("Deleted record for {}", record),
                None         => println!("{} not found.", name),
            }
        }

        Command::Update { name, salary, .. } => {
            // Log format: UPDATE,<hash>,<name>,<salary>
            let hash_val = jenkins_hash(name);
            logger.log_command(priority, &format!("UPDATE,{},{},{}", hash_val, name, salary));

            let mut records = table.write_lock(priority);
            let result = table_update(&mut records, name, *salary);
            table.log_write_released(priority);
            drop(records);

            match result {
                Some((h, old, new)) => println!("Updated record {} from {} to {}", h, old, new),
                None => {
                    println!("Update failed. Entry {} not found.", hash_val);
                }
            }
        }

        Command::Search { name, .. } => {
            // Log format: SEARCH,<hash>,<name>
            let hash_val = jenkins_hash(name);
            logger.log_command(priority, &format!("SEARCH,{},{}", hash_val, name));

            let records = table.read_lock(priority);
            let result  = table_search(&records, name);
            table.log_read_released(priority);
            drop(records);

            match result {
                Some(record) => println!("Found: {}", record),
                None         => println!("{} not found.", name),
            }
        }

        Command::Print { .. } => {
            logger.log_command(priority, "PRINT");

            let records = table.read_lock(priority);
            let output  = table_print(&records);
            table.log_read_released(priority);
            drop(records);

            print!("{}", output);
        }
    }
}
