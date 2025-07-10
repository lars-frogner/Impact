#!/bin/bash
set -e

OUTPUT_DIR=./dist/debug_cranelift
LOG_LEVEL=${1:-info}

OUTPUT_DIR=$OUTPUT_DIR \
CRANELIFT=1 \
ROC_DEBUG=1 \
RUST_DEBUG=1 \
roc build.roc

RUST_LOG="$LOG_LEVEL,naga=error,wgpu_core=error,wgpu_hal=error" \
RUST_BACKTRACE=1 \
$OUTPUT_DIR/snapshot_tester run -c config/config.ron
