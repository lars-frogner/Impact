#!/bin/bash
set -e

LOG_LEVEL=${1:-info}

mkdir -p lib roc_platform/lib
roc build.roc

RUST_LOG="$LOG_LEVEL,calloop=error,naga=error,wgpu_core=error,wgpu_hal=error" \
cargo run --manifest-path cli/Cargo.toml --release -- run -c config/config.ron
