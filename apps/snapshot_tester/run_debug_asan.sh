#!/bin/bash
set -e

OUTPUT_DIR=./dist/debug_asan
LOG_LEVEL=${1:-info}

OUTPUT_DIR=$OUTPUT_DIR \
ROC_DEBUG=1 \
RUST_DEBUG=1 \
ASAN=1 \
roc build.roc

LD_PRELOAD=/usr/lib/gcc/x86_64-linux-gnu/13/libasan.so \
ASAN_OPTIONS="\
detect_leaks=1:\
halt_on_error=1:\
allocator_may_return_null=1:\
detect_deadlocks=1:\
detect_stack_use_after_return=1" \
RUST_LOG="$LOG_LEVEL,naga=error,wgpu_core=error,wgpu_hal=error" \
RUST_BACKTRACE=1 \
LC_ALL=en_US.UTF-8  \
$OUTPUT_DIR/snapshot_tester run -c config/config.ron
