//! # Reader-Writer Lock Module
//!
//! This module provides a reader-writer lock (RwLock) wrapper that logs lock
//! acquisition and release events to the hash.log file.
//!
//! ## What is a Reader-Writer Lock?
//! A reader-writer lock allows multiple concurrent readers OR one exclusive writer,
//! but never both at the same time. This is more efficient than a simple mutex
//! when reads are much more frequent than writes.
//!
//! ## C vs Rust: Synchronization Primitives
//! In C (pthreads), you'd use:
//!   `pthread_rwlock_t lock;`
//!   `pthread_rwlock_rdlock(&lock);`
//!   `pthread_rwlock_wrlock(&lock);`
//!   `pthread_rwlock_unlock(&lock);`
//!
//! In Rust, `std::sync::RwLock<T>` wraps the DATA it protects, not just a raw
//! lock. You cannot access the data without going through the lock — the compiler
//! enforces this! The lock returns a "guard" object that automatically releases
//! the lock when it goes out of scope (RAII pattern). No `unlock()` needed!
//!
//! ## RAII (Resource Acquisition Is Initialization)
//! In C, you must manually call `pthread_rwlock_unlock()`. Forgetting to do so
//! causes a deadlock. In Rust, `RwLockReadGuard` and `RwLockWriteGuard` implement
//! the `Drop` trait — when the guard goes out of scope, the lock is automatically
//! released. You literally cannot forget to unlock.

use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::logger::Logger;

/// A logged reader-writer lock wrapper.
///
/// This wraps `std::sync::RwLock<T>` and adds logging of all lock/unlock events.
///
/// ## C vs Rust: Generics
/// The `<T>` makes this a generic type — it works for any data type T.
/// In C, you'd use `void*` pointers for genericity, losing type safety.
/// Rust generics are zero-cost: the compiler generates specialized code for each T.
///
/// ## Arc (Atomic Reference Counting)
/// `Arc<RwLock<T>>` is Rust's thread-safe equivalent of a shared pointer.
/// - `Arc` (Atomic Reference Count) allows multiple threads to share ownership.
/// - In C, you'd pass `pthread_rwlock_t*` around — but who owns it? Who frees it?
/// - With `Arc`, the lock is freed automatically when the last owner drops it.
pub struct LoggedRwLock<T> {
    inner: Arc<RwLock<T>>,
    logger: Arc<Logger>,
}

impl<T> LoggedRwLock<T> {
    /// Create a new LoggedRwLock wrapping the given data.
    pub fn new(data: T, logger: Arc<Logger>) -> Self {
        LoggedRwLock {
            inner: Arc::new(RwLock::new(data)),
            logger,
        }
    }

    /// Acquire a read lock, logging the acquisition and returning a guard.
    ///
    /// Multiple threads can hold read locks simultaneously.
    /// This blocks if a writer currently holds the lock.
    ///
    /// ## Returns
    /// `RwLockReadGuard` — a smart pointer to the data that releases the read lock
    /// when dropped. The caller cannot forget to release it.
    pub fn read_lock(&self, thread_priority: u32) -> RwLockReadGuard<'_, T> {
        // `unwrap()` panics if the lock is "poisoned" (a thread panicked while
        // holding it). In production code you'd handle this more gracefully,
        // but for this assignment it's acceptable.
        let guard = self.inner.read().unwrap();
        self.logger.log_read_acquired(thread_priority);
        guard
    }

    /// Release logging for read lock — called explicitly before guard is dropped
    /// so we can log the release at the right moment.
    pub fn log_read_released(&self, thread_priority: u32) {
        self.logger.log_read_released(thread_priority);
    }

    /// Acquire a write lock, logging the acquisition and returning a guard.
    ///
    /// Only one thread can hold a write lock at a time.
    /// This blocks if any readers OR another writer currently holds the lock.
    pub fn write_lock(&self, thread_priority: u32) -> RwLockWriteGuard<'_, T> {
        let guard = self.inner.write().unwrap();
        self.logger.log_write_acquired(thread_priority);
        guard
    }

    /// Release logging for write lock.
    pub fn log_write_released(&self, thread_priority: u32) {
        self.logger.log_write_released(thread_priority);
    }
}

/// Allow `LoggedRwLock` to be cloned by cloning the inner `Arc`.
///
/// Cloning an `Arc` just increments a reference count — it does NOT copy the data.
/// This is how multiple threads can share the same lock: they each hold a clone
/// of the Arc, all pointing to the same underlying RwLock.
///
/// In C, you'd share a pointer (`pthread_rwlock_t*`) — but then who frees it?
/// With Arc, the last clone to be dropped frees the memory automatically.
impl<T> Clone for LoggedRwLock<T> {
    fn clone(&self) -> Self {
        LoggedRwLock {
            inner: Arc::clone(&self.inner),
            logger: Arc::clone(&self.logger),
        }
    }
}
