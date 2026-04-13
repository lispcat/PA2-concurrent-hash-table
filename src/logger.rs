//! # Logger Module
//!
//! Thread-safe timestamped logging to `hash.log`.
//! Also tracks the total number of lock acquisitions and releases so the
//! summary lines ("Number of lock acquisitions: N") can be written at the end.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// Returns the current time as microseconds since the Unix epoch.
pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before Unix epoch")
        .as_micros() as u64
}

/// Thread-safe logger.
///
/// Wraps a `Mutex<File>` so concurrent threads can write without interleaving,
/// and an atomic pair of counters for lock acquisitions and releases.
pub struct Logger {
    file: Mutex<File>,
    /// Total number of lock acquisitions (reads + writes) across all threads.
    acquisitions: Mutex<u64>,
    /// Total number of lock releases (reads + writes) across all threads.
    releases: Mutex<u64>,
}

impl Logger {
    /// Create (or truncate) `hash.log` and return a new `Arc<Logger>`.
    pub fn new(path: &str) -> Arc<Self> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)
            .expect("failed to open hash.log");

        Arc::new(Logger {
            file: Mutex::new(file),
            acquisitions: Mutex::new(0),
            releases: Mutex::new(0),
        })
    }

    /// Write a line with a timestamp and "THREAD <priority> <message>" body.
    fn write(&self, thread_priority: u32, message: &str) {
        let ts = current_timestamp();
        let line = format!("{}: THREAD {} {}\n", ts, thread_priority, message);
        let mut file = self.file.lock().unwrap();
        let _ = file.write_all(line.as_bytes());
    }

    /// Log a command being executed.
    /// Format: `<ts>: THREAD <priority> <command_and_args>`
    pub fn log_command(&self, thread_priority: u32, command: &str) {
        self.write(thread_priority, command);
    }

    pub fn log_waiting(&self, thread_priority: u32) {
        self.write(thread_priority, "WAITING FOR MY TURN");
    }

    pub fn log_awakened(&self, thread_priority: u32) {
        self.write(thread_priority, "AWAKENED FOR WORK");
    }

    pub fn log_read_acquired(&self, thread_priority: u32) {
        *self.acquisitions.lock().unwrap() += 1;
        self.write(thread_priority, "READ LOCK ACQUIRED");
    }

    pub fn log_read_released(&self, thread_priority: u32) {
        *self.releases.lock().unwrap() += 1;
        self.write(thread_priority, "READ LOCK RELEASED");
    }

    pub fn log_write_acquired(&self, thread_priority: u32) {
        *self.acquisitions.lock().unwrap() += 1;
        self.write(thread_priority, "WRITE LOCK ACQUIRED");
    }

    pub fn log_write_released(&self, thread_priority: u32) {
        *self.releases.lock().unwrap() += 1;
        self.write(thread_priority, "WRITE LOCK RELEASED");
    }

    /// Return the current (acquisitions, releases) counts.
    pub fn lock_counts(&self) -> (u64, u64) {
        (
            *self.acquisitions.lock().unwrap(),
            *self.releases.lock().unwrap(),
        )
    }

    /// Write the summary counts to the log.
    /// Format matches the assignment's expected output:
    ///   Number of lock acquisitions: N
    ///   Number of lock releases: N
    pub fn log_counts(&self, acquisitions: u64, releases: u64) {
        let ts = current_timestamp();
        let mut file = self.file.lock().unwrap();
        let _ = file.write_all(
            format!("{}: Number of lock acquisitions: {}\n", ts, acquisitions).as_bytes(),
        );
        let _ =
            file.write_all(format!("{}: Number of lock releases: {}\n", ts, releases).as_bytes());
    }

    /// Write the final table to the log.
    /// `table_output` is the string from `table_print` (starts with "Current Database:\n").
    /// We replace the header with "Final Table:" to match the expected log format.
    pub fn log_final_table(&self, table_output: &str) {
        let ts = current_timestamp();
        let mut file = self.file.lock().unwrap();
        let _ = file.write_all(format!("{}: Final Table:\n", ts).as_bytes());
        // Skip the "Current Database:\n" header line, write the records as-is.
        for line in table_output.lines().skip(1) {
            let _ = file.write_all(format!("{}\n", line).as_bytes());
        }
    }
}
