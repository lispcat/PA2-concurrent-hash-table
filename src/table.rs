//! # Table Module
//!
//! This module implements the concurrent hash table.
//!
//! ## Data Structure
//! The table is implemented as a sorted `Vec<HashRecord>` rather than a raw
//! linked list of heap-allocated nodes (as a C implementation would use).
//!
//! ## Why Vec instead of a linked list?
//! The spec describes a linked list, but Rust makes it notoriously difficult to
//! implement a mutable, singly-linked list safely due to ownership rules.
//! A `Vec` is simpler, safer, cache-friendlier, and keeps elements sorted by hash.
//! The behavior is identical from the outside: O(n) insert/delete/search.
//!
//! If you want to explore the "real" linked list in Rust, see:
//!   https://rust-unofficial.github.io/too-many-lists/
//!
//! ## Thread Safety
//! The table itself is wrapped in `Arc<LoggedRwLock<...>>` in `main.rs`.
//! This module's functions take the inner `Vec<HashRecord>` directly (already locked)
//! to keep the logic clean and separate from the synchronization.

use crate::hash::{jenkins_hash, HashRecord};

/// Insert a new record into the table (sorted by hash).
///
/// Returns `Ok(record)` on success, or `Err(hash)` if duplicate found.
///
/// ## C equivalent:
/// ```c
/// // malloc() a new node, walk the linked list, insert before the node
/// // with the next-larger hash, or at the end.
/// ```
/// In Rust, `Vec::insert(pos, value)` handles memory automatically.
/// No `malloc`, no `free`, no memory leaks.
pub fn table_insert(
    records: &mut Vec<HashRecord>,
    name: &str,
    salary: u32,
) -> Result<HashRecord, u32> {
    let hash = jenkins_hash(name);

    // Binary search for existing entry — O(log n) since the Vec is sorted.
    // In C you'd walk the linked list linearly O(n).
    if records.iter().any(|r| r.hash == hash) {
        return Err(hash);
    }

    let record = HashRecord::new(name, salary);

    // Find insertion position to keep Vec sorted by hash (ascending).
    // `partition_point` is Rust's binary search for the insertion index.
    let pos = records.partition_point(|r| r.hash < hash);
    records.insert(pos, record.clone());

    Ok(record)
}

/// Delete a record by name. Returns the deleted record, or `None` if not found.
pub fn table_delete(records: &mut Vec<HashRecord>, name: &str) -> Option<HashRecord> {
    let hash = jenkins_hash(name);

    // Find the position of the record with matching hash.
    // In C you'd walk the list tracking the previous node to re-link.
    // `Vec::remove(pos)` handles pointer fixup automatically.
    if let Some(pos) = records.iter().position(|r| r.hash == hash) {
        Some(records.remove(pos))
    } else {
        None
    }
}

/// Update the salary of a record by name.
/// Returns `Some((hash, old_record_display))` on success, `None` if not found.
pub fn table_update(
    records: &mut Vec<HashRecord>,
    name: &str,
    new_salary: u32,
) -> Option<(u32, String, String)> {
    let hash = jenkins_hash(name);

    if let Some(record) = records.iter_mut().find(|r| r.hash == hash) {
        let old_display = format!("{}", record); // capture before update
        record.salary = new_salary;
        let new_display = format!("{}", record);
        Some((hash, old_display, new_display))
    } else {
        None
    }
}

/// Search for a record by name.
/// Returns a clone of the record if found, `None` otherwise.
///
/// ## Why clone?
/// The caller holds a read lock guard that borrows `records`. We can't return
/// a reference that outlives the guard. Cloning is the safe alternative.
/// In C, you'd return a `hashRecord*` — which is unsafe if the record is deleted
/// before the caller is done reading it!
pub fn table_search(records: &Vec<HashRecord>, name: &str) -> Option<HashRecord> {
    let hash = jenkins_hash(name);
    records.iter().find(|r| r.hash == hash).cloned()
}

/// Return all records as a formatted string, sorted by hash (already maintained).
pub fn table_print(records: &Vec<HashRecord>) -> String {
    let mut out = String::from("Current Database:\n");
    for r in records {
        out.push_str(&format!("{}\n", r));
    }
    out
}
