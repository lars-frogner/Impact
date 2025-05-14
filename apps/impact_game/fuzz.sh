#!/bin/bash
set -e
mkdir -p lib roc_platform/lib
DEBUG=1 FUZZING=1 roc build.roc

cargo build --manifest-path cli/Cargo.toml --release --features fuzzing

LD_PRELOAD=/usr/lib/gcc/x86_64-linux-gnu/13/libasan.so \
ASAN_OPTIONS="\
detect_leaks=1:\
halt_on_error=1:\
allocator_may_return_null=1:\
detect_deadlocks=1:\
detect_stack_use_after_return=1:\
strict_string_checks=1" \
RUST_BACKTRACE=1 \
./cli/target/release/impact_game_cli fuzz "$@"
