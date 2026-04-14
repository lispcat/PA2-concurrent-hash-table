# PA#2 Concurrent Hash Table

## About

This is a group project for the COP4600, Spring 2026 Operating Systems class.
- Group <TODO>
- Members: <TODO>
- This program is written in Rust with the help of an LLM, as per assignment instructions, and implements a concurrent hash table using reader-writer locks and condition variables.

---

## Project Structure

```
concurrent_hash_table/
├── Cargo.toml          # Rust package manifest (like a Makefile for dependencies)
├── Makefile            # Convenience wrapper around cargo commands
├── commands.txt        # Input file: thread count and operations
├── hash.log            # Output: timestamped lock/event diagnostics (generated at runtime)
└── src/
    ├── main.rs         # Entry point: thread spawning, turn ordering, command dispatch
    ├── hash.rs         # HashRecord struct + Jenkins one-at-a-time hash function
    ├── table.rs        # Hash table operations: insert, delete, update, search, print
    ├── logger.rs       # Thread-safe timestamped logger → hash.log
    ├── rwlock.rs       # Logged RwLock wrapper (logs acquire/release events)
    └── commands.rs     # Parser for commands.txt → typed Command enum
```

---

## Building and Running

```sh
make          # debug build
make run      # build + run (reads commands.txt from current directory)
make release  # optimized build
make test     # run unit tests
make clean    # remove build artifacts and hash.log
make doc      # generate and open HTML docs
```

---

## commands.txt Format

The first line specifies the total thread count:
```
threads,<count>,<unused>
```

Subsequent lines are operations, one per thread, with **priority as the last field**:

| Command | Format | Description |
|---|---|---|
| insert | `insert,<name>,<salary>,<priority>` | Insert new record |
| delete | `delete,<name>,<unused>,<priority>` | Delete record by name |
| update | `update,<name>,<new_salary>,<priority>` | Update salary by name |
| search | `search,<name>,<unused>,<priority>` | Search and print record |
| print  | `print,<unused>,<unused>,<priority>` | Print all records sorted by hash |

---

## Architecture

### Threading Model

Each command line becomes **one thread**. Two synchronization mechanisms work together:

```
Condition Variable (Ordering)        RwLock (Data Protection)
────────────────────────────         ────────────────────────
Thread waits for its "turn"    →     Thread computes hash (concurrent!)
Thread signals next thread     →     Thread acquires read or write lock
                                     Thread accesses the table
                                     Thread releases lock
```

The condvar ensures operations **start** in priority order. After signaling the next thread, both can run concurrently — the RwLock prevents data corruption during actual table access.

### Why Both?

- **Without condvar**: Operations happen in random thread-scheduling order — unpredictable output.
- **Without RwLock**: Multiple threads modifying the Vec simultaneously = data corruption.
- **With both**: Deterministic ordering AND safe concurrent access.

---

## Key Rust vs. C Concepts

### Ownership & Memory Safety

```c
// C: You allocate, you free. Forget → memory leak. Double free → crash.
hashRecord *node = malloc(sizeof(hashRecord));
// ... use node ...
free(node);
```

```rust
// Rust: Values are freed automatically when they go out of scope.
// The compiler tracks ownership — no malloc, no free, no leaks, no dangling pointers.
let record = HashRecord::new("Alice", 75000);
// record is freed here automatically
```

### Shared State Across Threads

```c
// C: Raw pointer + manual lock. Compiler doesn't enforce the lock is held.
hashRecord *table;
pthread_rwlock_t lock;
pthread_rwlock_wrlock(&lock);
table->salary = 90000; // What if you forget the lock?
pthread_rwlock_unlock(&lock);
```

```rust
// Rust: The data lives INSIDE the lock. You CANNOT access it without locking.
// The compiler enforces this at compile time — no runtime overhead for the check.
let table: Arc<RwLock<Vec<HashRecord>>> = Arc::new(RwLock::new(Vec::new()));
let mut guard = table.write().unwrap(); // acquire write lock
guard[0].salary = 90000;               // access data through the guard
// guard dropped here → write lock automatically released (RAII)
```

### Condition Variables

```c
// C: cond and mutex are separate objects you must manage carefully
pthread_mutex_t lock = PTHREAD_MUTEX_INITIALIZER;
pthread_cond_t  cond = PTHREAD_COND_INITIALIZER;
pthread_mutex_lock(&lock);
while (current_turn != my_id) pthread_cond_wait(&cond, &lock);
pthread_mutex_unlock(&lock);
```

```rust
// Rust: Condvar is paired with a Mutex in a tuple, shared via Arc
let pair = Arc::new((Mutex::new(0u32), Condvar::new()));
let (lock, cvar) = &*pair;
let mut turn = lock.lock().unwrap();       // MutexGuard (auto-unlocks on drop)
while *turn != my_id {
    turn = cvar.wait(turn).unwrap();       // atomically unlock + sleep
}
// No explicit unlock — the MutexGuard drops here
```

### Enums (Algebraic Data Types)

```c
// C: integer constants + void* or union — no type safety
#define CMD_INSERT 1
#define CMD_DELETE 2
struct Command { int type; char name[50]; int salary; };
// Nothing stops you from reading salary on a DELETE command
```

```rust
// Rust: Each enum variant carries exactly the data it needs
enum Command {
    Insert { name: String, salary: u32, priority: u32 },
    Delete { name: String, priority: u32 },
    // ... no salary field on Delete — it simply doesn't exist
}
// match forces you to handle every case:
match command {
    Command::Insert { name, salary, .. } => { /* ... */ }
    Command::Delete { name, .. } => { /* ... */ }
    // Compiler error if you miss a variant!
}
```

---

## Hash Function

Jenkins's one-at-a-time hash, ported from C with explicit wrapping arithmetic:

```rust
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
```

> In C, unsigned integer overflow wraps silently. In Rust debug builds, overflow panics.
> We use `wrapping_add` / `wrapping_shl` to make the wrapping intent explicit.

---

## Log File (hash.log)

Every lock operation and command is logged with microsecond timestamps:

```
1721428978841092: THREAD 0,INSERT,Shigeru Miyamoto,85000
1721428978841093: THREAD 0WRITE LOCK ACQUIRED
1721428978841095: THREAD 0WRITE LOCK RELEASED
1721428978841096: THREAD 1WAITING FOR MY TURN
1721428978841097: THREAD 1AWAKENED FOR WORK
```

---

## Future Work

- [ ] Add `clap` for command-line argument parsing (e.g., custom input file path)
- [ ] Add integration tests using `assert_cmd` crate
- [ ] Implement true linked list using `Box<Node>` for learning purposes
- [ ] Benchmark with `criterion` crate
