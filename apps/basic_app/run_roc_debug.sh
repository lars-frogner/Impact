#!/bin/bash
set -e

OUTPUT_DIR=./dist/roc_debug
LOG_LEVEL=${1:-info}

OUTPUT_DIR=$OUTPUT_DIR \
ROC_DEBUG=1 \
roc build.roc

RUST_LOG="$LOG_LEVEL,naga=error,wgpu_core=error,wgpu_hal=error" \
RUST_BACKTRACE=1 \
$OUTPUT_DIR/basic_app run -c config/config.ron
