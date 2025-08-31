#!/bin/bash
cargo build --release --features "cli,benchmark,unchecked" --bin impact
valgrind --tool=cachegrind ./target/release/impact benchmark "$@"
valgrind --tool=callgrind --collect-jumps=yes --dump-instr=yes --simulate-cache=yes ./target/release/impact benchmark "$@"
