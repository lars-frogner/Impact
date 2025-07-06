#!/bin/bash
cargo build --release --features "cli,profiling,unchecked" --bin impact
valgrind --tool=cachegrind ./target/release/impact profile "$@"
valgrind --tool=callgrind --collect-jumps=yes --dump-instr=yes --simulate-cache=yes ./target/release/impact profile "$@"
