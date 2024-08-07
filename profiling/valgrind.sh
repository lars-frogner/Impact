#!/bin/bash
RUSTFLAGS="-g" cargo build --release --bin profile
valgrind --tool=cachegrind ./target/release/profile "$@"
valgrind --tool=callgrind --collect-jumps=yes --dump-instr=yes --simulate-cache=yes ./target/release/profile "$@"