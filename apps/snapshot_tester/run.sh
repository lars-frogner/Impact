#!/bin/bash
set -e

OUTPUT_DIR=./dist/release
LOG_LEVEL=${1:-info}

OUTPUT_DIR=$OUTPUT_DIR \
roc build.roc

RUST_LOG="$LOG_LEVEL,calloop=error,naga=error,wgpu_core=error,wgpu_hal=error" \
$OUTPUT_DIR/snapshot_tester run -c config/config.ron
