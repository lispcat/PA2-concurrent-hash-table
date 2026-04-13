# Makefile for concurrent_hash_table
# A thin wrapper around cargo so you don't need to remember cargo subcommands.
#
# Targets:
#   make build      — compile in debug mode (fast compile, slow binary)
#   make run        — compile and run (reads commands.txt in current dir)
#   make release    — compile with optimisations (slow compile, fast binary)
#   make test       — run all unit and integration tests
#   make clean      — remove build artifacts and generated log files
#   make doc        — build HTML documentation and open in browser

.PHONY: build run release test clean doc

# Default target: compile only (no run).  Named `build`, not `all`, because
# there is no single "all" artifact — `cargo run` vs `cargo test` produce
# different things and shouldn't be conflated.
build:
	cargo build

run:
	cargo run

release:
	cargo build --release

test:
	cargo test

test-v:
	FULL_DIFF=1
	cargo test

clean:
	cargo clean
	rm -f hash.log

doc:
	cargo doc --open
