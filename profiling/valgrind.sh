#!/bin/bash
RUSTFLAGS="$RUSTFLAGS -g -Awarnings" cargo build --release --features profiling --bin profile
valgrind --tool=cachegrind ./target/release/profile "$@"
valgrind --tool=callgrind --collect-jumps=yes --dump-instr=yes --simulate-cache=yes ./target/release/profile "$@"
