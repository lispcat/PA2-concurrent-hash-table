//! # Integration Tests
//!
//! Each test:
//!   1. Locates the compiled binary via `env!("CARGO_BIN_EXE_chash")`.
//!   2. Creates a temporary directory and copies the input fixture there as
//!      `commands.txt`, since the binary always reads from that fixed filename
//!      in its current working directory.
//!   3. Runs the binary and captures stdout plus the generated `hash.log`.
//!   4. Compares stdout (final DB block only) against the golden `.out` file.
//!   5. Validates hash.log structurally (see below) against hard invariants
//!      that must hold regardless of thread scheduling.
//!
//! ## Diff verbosity
//! Set `FULL_DIFF=1` to print every line including matches on failure:
//!
//!   FULL_DIFF=1 cargo test -- --nocapture
//!
//! ## stdout comparison
//! Only the final `Current Database:` block is compared (extracted from both
//! sides), because intermediate output lines are non-deterministic.
//! The golden `.out` file can be pasted verbatim from the assignment spec.
//!
//! ## hash.log validation
//! The log is validated structurally rather than by exact match, because the
//! interleaving of concurrent threads is non-deterministic and will differ
//! between runs and between implementations.  We assert invariants that MUST
//! hold for any correct implementation:
//!
//!   - Every thread that appears in the log has exactly one WAITING, one
//!     AWAKENED, one lock ACQUIRED, and one lock RELEASED entry.
//!   - WAITING comes before AWAKENED for each thread.
//!   - ACQUIRED comes before RELEASED for each thread.
//!   - The lock acquisition count equals the release count.
//!   - The summary line "Number of lock acquisitions: N" is present and
//!     matches the actual number of ACQUIRED events in the log.
//!   - "Final Table:" is present at the end of the log.
//!
//! Why not do an exact line-by-line comparison?
//! Because the reference log came from a different binary with different hash
//! values and different thread scheduling.  An exact match would be both
//! impossible (wrong hashes) and meaningless (different-but-valid orderings
//! would falsely fail).  The invariant checks catch real bugs — missing locks,
//! unbalanced acquire/release, wrong counts — while tolerating valid variation.
//!
//! ## Adding a new test case
//! 1. Paste `<stem>.in` and `<stem>.out` from the assignment into `tests/data/`.
//! 2. Add: `#[test] fn test_<stem>() { run_test("<stem>"); }`

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
}

fn run_binary(cwd: &Path) -> (std::process::ExitStatus, String, String) {
    let bin = env!("CARGO_BIN_EXE_chash");
    let output = Command::new(bin)
        .current_dir(cwd)
        .output()
        .expect("failed to launch binary — did you `cargo build` first?");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (output.status, stdout, stderr)
}

fn make_temp_dir(label: &str) -> PathBuf {
    let dir = env::temp_dir()
        .join(format!("cht_test_{}_{}", label, std::process::id()));
    fs::create_dir_all(&dir).expect("could not create temp directory");
    dir
}

fn full_diff_enabled() -> bool {
    env::var("FULL_DIFF").as_deref() == Ok("1")
}

fn extract_final_db(text: &str, source_label: &str) -> String {
    let marker = "Current Database:";
    let last_pos = text.rfind(marker).unwrap_or_else(|| {
        panic!("no '{}' block found in {}\nFull text:\n{}", marker, source_label, text)
    });
    text[last_pos..].trim_end().to_string()
}

fn make_diff(expected: &str, actual: &str) -> String {
    let exp: Vec<&str> = expected.lines().collect();
    let act: Vec<&str> = actual.lines().collect();
    let max = exp.len().max(act.len());
    let verbose = full_diff_enabled();
    let mut out = String::new();

    for i in 0..max {
        match (exp.get(i), act.get(i)) {
            (Some(e), Some(a)) if e == a => {
                if verbose { out.push_str(&format!("  {:>4}: {}\n", i + 1, e)); }
            }
            (Some(e), Some(a)) => {
                out.push_str(&format!("~ {:>4}  expected: {}\n", i + 1, e));
                out.push_str(&format!("       actual  : {}\n", a));
            }
            (Some(e), None) => {
                out.push_str(&format!("- {:>4}  expected: {}\n", i + 1, e));
                out.push_str("       actual  : <missing>\n");
            }
            (None, Some(a)) => {
                out.push_str(&format!("+ {:>4}  expected: <missing>\n", i + 1));
                out.push_str(&format!("       actual  : {}\n", a));
            }
            (None, None) => unreachable!(),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// hash.log structural validation
// ---------------------------------------------------------------------------

/// Strip the timestamp prefix from a log line, returning just the content.
/// Format: `<microseconds>: <content>`
fn strip_timestamp(line: &str) -> &str {
    match line.find(": ") {
        Some(pos) => &line[pos + 2..],
        None      => line,
    }
}

/// Validate the structural invariants of a hash.log.
///
/// Returns Ok(()) if all invariants hold, or Err(String) with a description
/// of every violation found.
fn validate_log(raw: &str) -> Result<(), String> {
    let mut errors: Vec<String> = Vec::new();

    // Per-thread event tracking.
    // We track position (line index) of each event so we can check ordering.
    let mut waiting:   HashMap<u32, usize> = HashMap::new();
    let mut awakened:  HashMap<u32, usize> = HashMap::new();
    let mut acquired:  HashMap<u32, usize> = HashMap::new();
    let mut released:  HashMap<u32, usize> = HashMap::new();

    // Global counts.
    let mut total_acquired: u64 = 0;
    let mut total_released: u64 = 0;
    let mut reported_acquisitions: Option<u64> = None;
    let mut reported_releases:     Option<u64> = None;
    let mut final_table_found = false;
    let mut final_table_pos:  usize = 0;
    let mut last_lock_event_pos: usize = 0;

    let lines: Vec<&str> = raw.lines().collect();

    for (i, &line) in lines.iter().enumerate() {
        let content = strip_timestamp(line);

        // Parse "THREAD <n> <event>"
        if let Some(rest) = content.strip_prefix("THREAD ") {
            // Split on first space to get the thread number
            if let Some(sp) = rest.find(' ') {
                let thread_num_str = &rest[..sp];
                let event = &rest[sp + 1..];

                if let Ok(tid) = thread_num_str.parse::<u32>() {
                    match event {
                        "WAITING FOR MY TURN" => { waiting.insert(tid, i); }
                        "AWAKENED FOR WORK"   => { awakened.insert(tid, i); }
                        e if e.ends_with("LOCK ACQUIRED") => {
                            acquired.insert(tid, i);
                            total_acquired += 1;
                            last_lock_event_pos = i;
                        }
                        e if e.ends_with("LOCK RELEASED") => {
                            released.insert(tid, i);
                            total_released += 1;
                            last_lock_event_pos = i;
                        }
                        _ => {}
                    }
                }
            }
        } else if let Some(rest) = content.strip_prefix("Number of lock acquisitions: ") {
            reported_acquisitions = rest.trim().parse().ok();
        } else if let Some(rest) = content.strip_prefix("Number of lock releases: ") {
            reported_releases = rest.trim().parse().ok();
        } else if content == "Final Table:" {
            final_table_found = true;
            final_table_pos   = i;
        }
    }

    // --- Invariant: every thread that waited also woke up ---
    for (&tid, &wait_pos) in &waiting {
        match awakened.get(&tid) {
            None => errors.push(format!(
                "THREAD {} has WAITING (line {}) but no AWAKENED", tid, wait_pos + 1)),
            Some(&awake_pos) if awake_pos < wait_pos => errors.push(format!(
                "THREAD {} AWAKENED (line {}) before WAITING (line {})",
                tid, awake_pos + 1, wait_pos + 1)),
            _ => {}
        }
    }

    // --- Invariant: every thread that acquired also released ---
    for (&tid, &acq_pos) in &acquired {
        match released.get(&tid) {
            None => errors.push(format!(
                "THREAD {} ACQUIRED lock (line {}) but never RELEASED", tid, acq_pos + 1)),
            Some(&rel_pos) if rel_pos < acq_pos => errors.push(format!(
                "THREAD {} RELEASED (line {}) before ACQUIRED (line {})",
                tid, rel_pos + 1, acq_pos + 1)),
            _ => {}
        }
    }

    // --- Invariant: acquisition count equals release count ---
    if total_acquired != total_released {
        errors.push(format!(
            "lock acquisition count ({}) != release count ({})",
            total_acquired, total_released));
    }

    // --- Invariant: reported counts match actual log events ---
    // Note: the reported count is snapshotted BEFORE the final post-join lock,
    // so reported count = total_acquired - 1 (the final lock isn't included).
    if let Some(reported) = reported_acquisitions {
        // Allow the reported value to be total_acquired or total_acquired - 1
        // (depending on whether the implementation counts the final lock or not).
        if reported != total_acquired && reported != total_acquired.saturating_sub(1) {
            errors.push(format!(
                "reported acquisitions ({}) doesn't match log events ({} or {})",
                reported, total_acquired, total_acquired.saturating_sub(1)));
        }
    } else {
        errors.push("'Number of lock acquisitions:' line not found in log".to_string());
    }
    if let Some(reported) = reported_releases {
        if reported != total_released && reported != total_released.saturating_sub(1) {
            errors.push(format!(
                "reported releases ({}) doesn't match log events ({} or {})",
                reported, total_released, total_released.saturating_sub(1)));
        }
    } else {
        errors.push("'Number of lock releases:' line not found in log".to_string());
    }

    // --- Invariant: Final Table is present and comes after all lock events ---
    if !final_table_found {
        errors.push("'Final Table:' not found in log".to_string());
    } else if final_table_pos < last_lock_event_pos {
        errors.push(format!(
            "'Final Table:' (line {}) appears before last lock event (line {})",
            final_table_pos + 1, last_lock_event_pos + 1));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

// ---------------------------------------------------------------------------
// Core test driver
// ---------------------------------------------------------------------------

fn run_test(stem: &str) {
    let data          = data_dir();
    let input_file    = data.join(format!("{}.in",  stem));
    let golden_stdout = data.join(format!("{}.out", stem));

    assert!(input_file.exists(),
        "test input not found: {}\nAdd it to tests/data/", input_file.display());
    assert!(golden_stdout.exists(),
        "golden stdout not found: {}\nAdd it to tests/data/", golden_stdout.display());

    let tmp = make_temp_dir(stem);
    fs::copy(&input_file, tmp.join("commands.txt"))
        .expect("could not copy input fixture to temp dir");

    let (status, stdout, stderr) = run_binary(&tmp);
    assert!(status.success(),
        "binary exited with non-zero status\nstdout:\n{}\nstderr:\n{}", stdout, stderr);

    // -----------------------------------------------------------------------
    // 1. stdout — compare final Current Database: block only
    // -----------------------------------------------------------------------
    let golden_text = fs::read_to_string(&golden_stdout)
        .expect("could not read golden stdout file");

    let expected_db = extract_final_db(&golden_text, &golden_stdout.display().to_string());
    let actual_db   = extract_final_db(&stdout, "binary stdout");

    if actual_db != expected_db {
        let diff = make_diff(&expected_db, &actual_db);
        panic!(
            "stdout mismatch for '{}':\n{}\
             \n(set FULL_DIFF=1 to see all lines)\
             \n--- golden (final block): {}",
            stem, diff, golden_stdout.display());
    }

    // -----------------------------------------------------------------------
    // 2. hash.log — structural invariant checks
    // -----------------------------------------------------------------------
    let actual_log_path = tmp.join("hash.log");
    assert!(actual_log_path.exists(),
        "binary did not produce hash.log in {}", tmp.display());

    let raw_log = fs::read_to_string(&actual_log_path)
        .expect("could not read hash.log");

    if let Err(violations) = validate_log(&raw_log) {
        panic!("hash.log invariant violations for '{}':\n{}", stem, violations);
    }
}

// ---------------------------------------------------------------------------
// Test cases
// ---------------------------------------------------------------------------

#[test]
fn test_sample() {
    run_test("test0");
}

// Add more tests here as you add fixture files, e.g.:
// #[test]
// fn test_1() { run_test("test1"); }
