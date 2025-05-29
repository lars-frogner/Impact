#!/bin/bash
set -e
mkdir -p lib roc_platform/lib
CRANELIFT=1 DEBUG=1 roc build.roc
RUST_LOG=debug,naga=error,wgpu_core=error,wgpu_hal=error \
RUST_BACKTRACE=1 \
cargo run --manifest-path cli/Cargo.toml -- run -c config/config.ron
