# PA#2 Concurrent Hash Table — Rust Documentation

This document explains the Rust-specific concepts used in this project, written for someone who knows C but is new to Rust. Every place where Rust differs significantly from C is called out explicitly.

---

## Table of Contents

1. [Why Rust?](#why-rust)
2. [Ownership and Memory Safety](#ownership-and-memory-safety)
3. [Structs and the Display Trait](#structs-and-the-display-trait)
4. [Enums as Algebraic Data Types](#enums-as-algebraic-data-types)
5. [Error Handling with Result](#error-handling-with-result)
6. [Threads](#threads)
7. [Arc — Shared Ownership Across Threads](#arc--shared-ownership-across-threads)
8. [Mutex and RwLock — Protecting Shared Data](#mutex-and-rwlock--protecting-shared-data)
9. [Condition Variables](#condition-variables)
10. [RAII — Automatic Resource Cleanup](#raii--automatic-resource-cleanup)
11. [Closures](#closures)
12. [The Module System](#the-module-system)
13. [The Hash Function — Wrapping Arithmetic](#the-hash-function--wrapping-arithmetic)

---

## Why Rust?

In C, you are responsible for every byte of memory you allocate. Forget to `free()` something and you leak memory. Free it twice and you corrupt the heap. Access it after freeing and you have undefined behavior. In a multi-threaded program, two threads writing to the same memory without a lock causes a data race — also undefined behavior, and one of the hardest bugs to reproduce and fix.

Rust eliminates these entire categories of bugs **at compile time**. The Rust compiler enforces rules about ownership, borrowing, and thread safety. If your program compiles, it is guaranteed to be free of:

- Memory leaks (in safe code)
- Use-after-free
- Double-free
- Data races

This project is a good demonstration of that: we spawn 60 threads that all share a hash table, and the compiler verifies that every access is properly synchronized — without us having to manually audit every code path.

---

## Ownership and Memory Safety

### The Ownership Rules

Every value in Rust has exactly one *owner*. When the owner goes out of scope, the value is freed. There is no garbage collector and no manual `free()`.

```c
// C: you must free manually
hashRecord *node = malloc(sizeof(hashRecord));
node->salary = 75000;
// ... use node ...
free(node);  // forget this → memory leak
```

```rust
// Rust: freed automatically when `record` goes out of scope
let record = HashRecord::new("Alice", 75000);
// record is dropped (freed) here, at the end of the block
```

### Borrowing

Instead of passing pointers around (which in C can dangle or alias unsafely), Rust has *references*. A reference is a guaranteed-valid pointer. The compiler checks that references never outlive the data they point to.

```c
// C: nothing stops you from returning a pointer to a local variable (dangling pointer)
hashRecord* dangerous() {
    hashRecord r = {0};
    return &r;  // r is destroyed here — caller has garbage
}
```

```rust
// Rust: the compiler refuses to compile this
fn dangerous() -> &HashRecord {
    let r = HashRecord::new("Alice", 75000);
    &r  // compile error: `r` does not live long enough
}
```

In this project, functions like `table_search` take `&Vec<HashRecord>` (a read-only reference) and `table_insert` takes `&mut Vec<HashRecord>` (an exclusive mutable reference). The compiler ensures that while a mutable reference exists, no other references to the same data can exist — enforcing the same invariant that a write lock gives you, but at compile time.

---

## Structs and the Display Trait

In C, a struct is just a bag of fields. In Rust, you can attach behavior to a struct using `impl` blocks, and you can opt into standard interfaces (called *traits*) that let your type work with the rest of the language.

```c
typedef struct hash_struct {
    uint32_t hash;
    char name[50];
    uint32_t salary;
    struct hash_struct *next;
} hashRecord;

// You write a custom print function
void print_record(hashRecord *r) {
    printf("%u,%s,%u\n", r->hash, r->name, r->salary);
}
```

```rust
pub struct HashRecord {
    pub hash:   u32,
    pub name:   String,  // heap-allocated, bounds-checked, no buffer overflow
    pub salary: u32,
    // no `next` pointer — we use Vec<HashRecord> instead of a linked list
}

// Implement the Display trait so println!("{}", record) just works
impl std::fmt::Display for HashRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{},{},{}", self.hash, self.name, self.salary)
    }
}
```

`String` replaces `char name[50]`. It is heap-allocated, UTF-8, and dynamically sized. There is no fixed buffer to overflow.

We also derive common traits automatically:

```rust
#[derive(Clone, Debug)]
pub struct HashRecord { ... }
```

`#[derive(Clone)]` generates a `clone()` method (like a deep copy). `#[derive(Debug)]` generates a `{:?}` formatter for debugging. In C you'd write these by hand.

---

## Enums as Algebraic Data Types

C enums are just integers with names. Rust enums are *algebraic data types* — each variant can carry different data, and you are forced to handle every case.

```c
// C: type safety is lost. Nothing prevents treating a DELETE as an INSERT.
#define CMD_INSERT 1
#define CMD_DELETE 2
struct Command { int type; char name[50]; uint32_t salary; int priority; };
```

```rust
// Rust: each variant carries exactly what it needs, nothing more.
pub enum Command {
    Insert { name: String, salary: u32, priority: u32 },
    Delete { name: String, priority: u32 },           // no salary field
    Update { name: String, salary: u32, priority: u32 },
    Search { name: String, priority: u32 },
    Print  { priority: u32 },
}
```

`match` replaces `switch`. Unlike C's `switch`, Rust's `match` is exhaustive — if you forget a variant, the code won't compile:

```rust
match command {
    Command::Insert { name, salary, .. } => { /* insert logic */ }
    Command::Delete { name, .. }         => { /* delete logic */ }
    Command::Update { name, salary, .. } => { /* update logic */ }
    Command::Search { name, .. }         => { /* search logic */ }
    Command::Print  { .. }               => { /* print logic  */ }
    // forgetting any of these is a compile error
}
```

---

## Error Handling with Result

C signals errors through return codes or `NULL` pointers, which are easy to ignore. Rust uses `Result<T, E>`, a type that is *either* a success value `Ok(T)` or an error `Err(E)`. The compiler forces you to handle it.

```c
// C: easy to ignore the return value
hashRecord *result = search(table, "Alice");
printf("%u\n", result->salary);  // crash if result is NULL
```

```rust
// Rust: you must handle both cases
match table_insert(&mut records, name, salary) {
    Ok(record) => println!("Inserted {}", record),
    Err(hash)  => println!("Insert failed. Entry {} is a duplicate.", hash),
}
```

In `table.rs`, `table_insert` returns `Result<HashRecord, u32>` — either the new record on success, or the hash of the duplicate on failure. It is impossible to accidentally use the error case as a success.

---

## Threads

In C, you spawn threads with `pthread_create`, passing a function pointer and a `void*` argument. The compiler has no way to check what you put in that `void*` or whether it is safe to share across threads.

```c
pthread_t tid;
pthread_create(&tid, NULL, worker_fn, (void*)arg);
pthread_join(tid, NULL);
```

In Rust, `thread::spawn` takes a *closure* (an anonymous function) that captures its environment. The `move` keyword transfers ownership of captured variables into the new thread. Crucially, the compiler checks that everything you send into a thread implements the `Send` trait — a compile-time guarantee that the type is safe to transfer across threads.

```rust
let handle = thread::spawn(move || {
    // `table_clone`, `logger_clone`, `turn_clone` are moved in here.
    // The compiler verified they are Send before allowing this.
    execute_command(&command, &table_clone, &logger_clone, priority);
});

// Wait for the thread to finish (like pthread_join)
handle.join().expect("a worker thread panicked");
```

`JoinHandle<()>` is the return type of `thread::spawn`. It is Rust's equivalent of `pthread_t`. We collect all handles in a `Vec` and join them after spawning all threads.

---

## Arc — Shared Ownership Across Threads

In C, when you want multiple threads to share data, you pass a pointer. There is no language-enforced rule about who owns it or who frees it.

Rust enforces *single ownership*, but that is too restrictive for sharing across threads. `Arc<T>` (Atomic Reference Count) is the solution. It is a smart pointer that multiple owners can hold. The underlying data is freed when the last `Arc` clone is dropped.

```c
// C: pass a raw pointer — who frees it? When?
pthread_create(&tid, NULL, worker, (void*)&shared_table);
```

```rust
// Rust: clone the Arc (increments a reference count, does NOT copy the data)
let table: Arc<LoggedRwLock<Vec<HashRecord>>> = Arc::new(...);

let table_clone = Arc::clone(&table);  // now two owners, same data
thread::spawn(move || {
    // table_clone is valid here; the data lives as long as any Arc does
});
// When the last Arc is dropped, the data is freed
```

We clone `Arc`s for the table, logger, and turn-state before every `thread::spawn` call. Each clone is a lightweight reference-count increment, not a copy of the data.

---

## Mutex and RwLock — Protecting Shared Data

### The C Way

In C, a lock and the data it protects are separate objects. Nothing in the language connects them:

```c
hashRecord *head = NULL;      // the data
pthread_rwlock_t lock;         // the lock — separate!

pthread_rwlock_wrlock(&lock);
// modify head...
pthread_rwlock_unlock(&lock);  // forget this → deadlock
// Nothing stops you from modifying head without holding the lock
```

### The Rust Way

In Rust, `Mutex<T>` and `RwLock<T>` *wrap* the data. You cannot access the data without going through the lock. This is enforced at compile time:

```rust
let table = RwLock::new(Vec::<HashRecord>::new());

// To read:
let records = table.read().unwrap();    // acquires read lock, returns a guard
// `records` derefs to &Vec<HashRecord>
// The lock is held as long as `records` lives

// To write:
let mut records = table.write().unwrap(); // acquires write lock
records.push(new_record);
// Lock is released when `records` is dropped
```

There is literally no way to access the `Vec` without calling `.read()` or `.write()` first. The lock cannot be "forgotten."

### Our LoggedRwLock

We wrap `RwLock<T>` in our own `LoggedRwLock<T>` struct (`rwlock.rs`) that logs every acquisition and release to `hash.log`. It is generic over `T`, so it works for any data type:

```rust
pub struct LoggedRwLock<T> {
    inner:  Arc<RwLock<T>>,
    logger: Arc<Logger>,
}
```

The `'_` lifetime annotation in the return types of `read_lock` and `write_lock`:

```rust
pub fn read_lock(&self, priority: u32) -> RwLockReadGuard<'_, T>
```

This tells the compiler (and the reader) that the returned guard borrows from `self` — it cannot outlive the `LoggedRwLock` that produced it.

---

## Condition Variables

A condition variable lets threads wait for a condition to become true without busy-looping. In C, the condvar and its associated mutex are separate objects. In Rust, they are paired in a tuple and shared via `Arc`:

```c
// C
pthread_mutex_t lock = PTHREAD_MUTEX_INITIALIZER;
pthread_cond_t  cond = PTHREAD_COND_INITIALIZER;

pthread_mutex_lock(&lock);
while (current_turn != my_id)
    pthread_cond_wait(&cond, &lock);  // atomically releases lock, sleeps
pthread_mutex_unlock(&lock);

// Signal:
pthread_mutex_lock(&lock);
current_turn++;
pthread_cond_broadcast(&cond);
pthread_mutex_unlock(&lock);
```

```rust
// Rust
let pair: Arc<(Mutex<u32>, Condvar)> = Arc::new((Mutex::new(0), Condvar::new()));
let (lock, cvar) = &*pair;

// Wait:
let mut current_turn = lock.lock().unwrap();  // acquires MutexGuard
while *current_turn != thread_id {
    // cvar.wait() atomically releases the mutex and sleeps.
    // On wakeup, it re-acquires the mutex and returns the new guard.
    current_turn = cvar.wait(current_turn).unwrap();
}
// MutexGuard dropped here → mutex released automatically

// Signal:
let mut current_turn = lock.lock().unwrap();
*current_turn += 1;
cvar.notify_all();  // == pthread_cond_broadcast
```

The key difference: `cvar.wait(guard)` takes ownership of the `MutexGuard` and returns a new one. You cannot call `wait` without holding the mutex, because the guard proves you hold it. In C, nothing stops you from calling `pthread_cond_wait` without holding the lock first — undefined behavior.

In this project, the condition variable controls **turn ordering**: each thread waits until `current_turn == thread_id`, then increments `current_turn` and broadcasts before doing its work. This lets threads overlap in execution (hashing, etc.) while still starting in priority order.

---

## RAII — Automatic Resource Cleanup

RAII (Resource Acquisition Is Initialization) means that a resource is tied to an object's lifetime. When the object is destroyed (goes out of scope), the resource is automatically released.

Rust uses RAII everywhere via the `Drop` trait. Lock guards are the most important example:

```rust
{
    let mut records = table.write_lock(priority);  // write lock acquired here
    table_insert(&mut records, name, salary);
    table.log_write_released(priority);
    drop(records);  // explicit drop — write lock released here
    // (we drop early so we can print without holding the lock)

    println!("Inserted ...");
}
```

In C, you must remember to call `pthread_rwlock_unlock()` on every exit path — including early returns and error branches. In Rust, you can call `drop()` explicitly, or simply let the guard go out of scope, and the lock is released. There is no way to forget.

---

## Closures

A closure is an anonymous function that can capture variables from the surrounding scope. Rust closures are similar to C function pointers + a manually-managed context struct, but far safer.

```c
// C: pass a function pointer + void* context
struct WorkerArgs { int thread_id; hashRecord **table; pthread_rwlock_t *lock; };
pthread_create(&tid, NULL, worker_fn, (void*)&args);
```

```rust
// Rust: the closure captures what it needs; `move` transfers ownership
let handle = thread::spawn(move || {
    // thread_id, table_clone, logger_clone are captured by value
    // The compiler checked they are safe to send across threads
    execute_command(&command, &table_clone, &logger_clone, priority);
});
```

The `move` keyword is required when spawning threads so that the closure owns its captured data. Without `move`, the closure would hold references — but references cannot be sent across threads because the referent might not live long enough.

---

## The Module System

In C, you separate code into files and use `#include` to bring in declarations. There is one global namespace.

In Rust, code is organized into *modules*. Each file is a module. You declare submodules in `main.rs` with `mod`:

```rust
mod commands;  // loads src/commands.rs
mod hash;      // loads src/hash.rs
mod logger;    // loads src/logger.rs
mod rwlock;    // loads src/rwlock.rs
mod table;     // loads src/table.rs
```

Items inside a module are private by default. You must explicitly mark things `pub` to expose them. `use` statements bring specific items into scope:

```rust
use commands::{parse_commands, Command};
use hash::{jenkins_hash, HashRecord};
```

This is stricter than C's `#include`, which dumps everything into the global namespace. In Rust, if you can see it, it was explicitly exported.

---

## The Hash Function — Wrapping Arithmetic

The Jenkins one-at-a-time hash uses arithmetic that intentionally overflows. In C, overflow of `unsigned` types wraps silently:

```c
uint32_t hash = 0;
hash += key[i];     // wraps silently if it overflows — defined behavior for unsigned
hash += hash << 10; // same
```

In Rust, integer overflow in **debug builds** causes a panic (a controlled crash with an error message). In **release builds** it wraps silently, matching C. For code that intentionally wraps, we use explicit wrapping methods so the behavior is the same in both build modes:

```rust
hash = hash.wrapping_add(byte as u32);
hash = hash.wrapping_add(hash.wrapping_shl(10));
hash ^= hash >> 6;
```

This makes the intent clear: we know this wraps, and we want it to. The C version relies on silent unsigned overflow; the Rust version documents the choice in the code itself.
