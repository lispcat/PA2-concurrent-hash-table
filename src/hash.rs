//! # Hash Module
//!
//! This module provides the core data structures and the Jenkins one-at-a-time
//! hash function used to map string keys to 32-bit integer hash values.
//!
//! ## C vs Rust: Structs
//! In C, you'd write:
//!   typedef struct hash_struct { uint32_t hash; char name[50]; ... } hashRecord;
//!
//! In Rust, structs are defined with `struct` and fields are private by default.
//! We derive common traits (Clone, Debug) automatically instead of writing them by hand.
//! There's no need for `typedef` — the struct name IS the type.

/// A single record in the concurrent hash table.
///
/// This corresponds directly to the C struct:
/// ```c
/// typedef struct hash_struct {
///     uint32_t hash;
///     char name[50];
///     uint32_t salary;
///     struct hash_struct *next;
/// } hashRecord;
/// ```
///
/// ## Key differences from C:
/// - `String` replaces `char name[50]` — Rust strings are heap-allocated, UTF-8,
///   and bounds-checked. No buffer overflows possible!
/// - There is no `next` pointer here. Instead of a raw pointer linked list,
///   we use `Vec<HashRecord>` in the table — simpler and safer.
/// - `#[derive(Clone, Debug)]` auto-generates clone and print functionality.
#[derive(Clone, Debug)]
pub struct HashRecord {
    /// The 32-bit hash of the `name` field, computed by Jenkins one-at-a-time.
    pub hash: u32,
    /// The name/key — up to 50 chars in the spec, but Rust Strings can be any length.
    pub name: String,
    /// Annual salary as an unsigned 32-bit integer (no negative salaries!).
    pub salary: u32,
}

impl HashRecord {
    /// Create a new HashRecord, computing the hash automatically from the name.
    ///
    /// In C you'd call `malloc()` and fill fields manually. In Rust, we just
    /// return a value — memory is managed automatically by ownership rules.
    pub fn new(name: &str, salary: u32) -> Self {
        let hash = jenkins_hash(name);
        HashRecord {
            hash,
            name: name.to_string(),
            salary,
        }
    }
}

/// Implement the Display trait so we can print records in the required format.
///
/// In C you'd write a `print_record(hashRecord *r)` function. In Rust,
/// implementing `std::fmt::Display` lets us use `println!("{}", record)` directly.
impl std::fmt::Display for HashRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{},{},{}", self.hash, self.name, self.salary)
    }
}

/// Computes Jenkins's one-at-a-time hash for a string key.
///
/// This is a direct port of the C reference implementation:
/// ```c
/// uint32_t jenkins_one_at_a_time_hash(const uint8_t* key, size_t length) {
///   size_t i = 0;
///   uint32_t hash = 0;
///   while (i != length) {
///     hash += key[i++];
///     hash += hash << 10;
///     hash ^= hash >> 6;
///   }
///   hash += hash << 3;
///   hash ^= hash >> 11;
///   hash += hash << 15;
///   return hash;
/// }
/// ```
///
/// ## C vs Rust: Integer arithmetic
/// In C, unsigned integer overflow wraps silently. In Rust, debug builds panic on
/// overflow. We use `wrapping_add`, `wrapping_shl` etc. to explicitly allow
/// wrapping, making our intent clear and avoiding surprises.
///
/// ## Why bytes?
/// `name.as_bytes()` gives us a `&[u8]` — a slice of bytes. This mirrors
/// `const uint8_t* key` in C, but with the length bundled in (no separate `size_t`).
pub fn jenkins_hash(name: &str) -> u32 {
    let mut hash: u32 = 0;
    for &byte in name.as_bytes() {
        hash = hash.wrapping_add(byte as u32);
        hash = hash.wrapping_add(hash.wrapping_shl(10));
        hash ^= hash >> 6;
    }
    hash = hash.wrapping_add(hash.wrapping_shl(3));
    hash ^= hash >> 11;
    hash = hash.wrapping_add(hash.wrapping_shl(15));
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jenkins_hash_known_values() {
        // Verify against known values from the spec
        assert_eq!(jenkins_hash("a"), 0xca2e9442);
        assert_eq!(
            jenkins_hash("The quick brown fox jumps over the lazy dog"),
            0x519e91f5
        );
    }
}
