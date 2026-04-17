# PA#2 Concurrent Hash Table

## Table of Contents

- [About](#about)
- [Organization](#organization)
- [Installing/Running](#installingrunning)
  - [Clone repository](#clone-repository)
  - [Compile](#compile)
  - [Optional: Running Tests](#optional-running-tests)
  - [Running the program](#running-the-program)
    - [Input file](#input-file)
    - [Makefile targets](#makefile-targets)
- [commands.txt Format](#commandstxt-format)
- [AI Attribution](#ai-attribution)
- [License](#license)

---

## About

This is a group project for the COP4600, Spring 2026 Operating Systems class.

- **Group Members:** Brianna, Yui, Joseph, Ryan, Michael (last names omitted for privacy).
- This program is written in Rust with the help of an LLM, as per assignment instructions, and implements a concurrent hash table using reader-writer locks and condition variables.

For a detailed explanation of the Rust-specific concepts used in this project (ownership, threads, locks, condition variables, and how they compare to C), see [DOCUMENTATION.md](DOCUMENTATION.md).

---

## Organization

The codebase is organized with modularity and test-driven development in mind. Source files under `./src/` are separated into modules, with `./src/main.rs` as the entry point. Integration tests live in `./tests/integration_test.rs` and use fixture pairs in `./tests/data/`.

```
.
├── Cargo.lock
├── Cargo.toml
├── Makefile
├── README.md
├── DOCUMENTATION.md      # Rust teaching documentation (see this for concept explanations)
├── commands.txt          # sample input file (binary reads from CWD at runtime)
├── src
│   ├── main.rs           # entry point: thread spawning, turn ordering, dispatch
│   ├── hash.rs           # HashRecord struct + Jenkins one-at-a-time hash
│   ├── table.rs          # insert, delete, update, search, print operations
│   ├── logger.rs         # thread-safe timestamped logger → hash.log
│   ├── rwlock.rs         # logged RwLock wrapper (logs acquire/release events)
│   └── commands.rs       # parser for commands.txt → typed Command enum
└── tests
    ├── data
    │   ├── test0.in
    │   └── test0.out
    └── integration_test.rs
```

---

## Installing/Running

### Clone repository

```sh
git clone https://github.com/lispcat/PA2-concurrent-hash-table.git
cd PA2-concurrent-hash-table
```

### Compile

Running `cargo build` (or `make build`) compiles the project and places the binary at `./target/debug/chash`.

```sh
cargo build
# or equivalently:
make build
```

### Optional: Running Tests

Run `cargo test` (or `make test`) to execute all unit and integration tests. If a test fails, a diff of the final database state will be shown.

```sh
cargo test
# or equivalently:
make test
```

### Running the program

The binary always reads `commands.txt` from its **current working directory** and writes `hash.log` there as well.

```sh
# From the repo root (with commands.txt present):
make run

# Or with cargo directly:
cargo run

# Or run the compiled binary from any directory containing commands.txt:
cd some/directory/with/commands.txt
/path/to/chash
```

Output is printed to stdout. Diagnostic lock events are written to `hash.log` in the same directory.

#### Input file

To run against a specific input file, copy it to the working directory as `commands.txt`:

```sh
cp tests/data/test0.in commands.txt
cargo run
```

#### Makefile targets

| Target          | Description                                       |
|-----------------|---------------------------------------------------|
| `make build`    | Compile in debug mode (fast compile)              |
| `make run`      | Compile and run (reads `commands.txt` from CWD)   |
| `make release`  | Compile with optimizations (faster binary)        |
| `make test`     | Run all unit and integration tests                |
| `make clean`    | Remove build artifacts and `hash.log`             |
| `make doc`      | Build and open HTML documentation                 |

---

## commands.txt Format

The first line specifies the total thread count:

```
threads,<count>,<unused>
```

Subsequent lines are operations, one per thread, with **priority as the last field**:

| Command  | Format                                | Description                       |
|----------|---------------------------------------|-----------------------------------|
| `insert` | `insert,<n>,<salary>,<priority>`      | Insert new record                 |
| `delete` | `delete,<n>,<unused>,<priority>`      | Delete record by name             |
| `update` | `update,<n>,<new_salary>,<priority>`  | Update salary by name             |
| `search` | `search,<n>,<unused>,<priority>`      | Search and print record           |
| `print`  | `print,<unused>,<unused>,<priority>`  | Print all records sorted by hash  |

---

## AI Attribution

This project was developed with the assistance of **Claude** (claude.ai, Anthropic), used as a pair-programming tool throughout development.

The AI was used for the following:

- **Initial project scaffolding** — generating the modular file structure (`hash.rs`, `table.rs`, `logger.rs`, `rwlock.rs`, `commands.rs`, `main.rs`) from the assignment specification.
- **Rust-specific idioms** — translating C concepts (pthreads, rwlocks, condvars, linked lists) into their idiomatic Rust equivalents (`std::sync::RwLock`, `Condvar`, `Arc`, `Vec`), including explaining ownership, borrowing, and RAII.
- **Integration test framework** — designing `tests/integration_test.rs` with structural invariant checks for `hash.log` and golden-file comparison for stdout.
- **Debugging** — identifying a command parser bug where `priority` was being read from the wrong column for `search` and `delete` commands, and fixing a `sed`-corrupted import line in `main.rs`.
- **Documentation** — drafting `DOCUMENTATION.md` as a teaching document explaining Rust thread-safety and memory-safety concepts for a C-familiar audience.

Prompts were iterative and detailed, providing the full assignment specification, sample input/output, and error messages at each step. All generated code was reviewed, tested against the provided sample data, and corrected where the output did not match the specification. The final hash values, thread ordering, lock counts, and database state were verified to match the expected output before submission.

---

## License

This project is licensed under the GNU General Public License v3.0. See the `LICENSE` file for details.
