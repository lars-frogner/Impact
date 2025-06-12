#!/bin/bash
set -e

LOG_LEVEL=${1:-info}

mkdir -p lib roc_platform/lib
DEBUG=1 roc build.roc
RUST_LOG="$LOG_LEVEL,naga=error,wgpu_core=error,wgpu_hal=error" \
RUST_BACKTRACE=1 \
cargo run --manifest-path cli/Cargo.toml -- run -c config/config.ron
