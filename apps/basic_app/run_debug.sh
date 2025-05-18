#!/bin/bash
set -e
mkdir -p lib roc_platform/lib
DEBUG=1 roc build.roc
RUST_LOG=debug,impact::thread=info,impact::thread=info,impact::scheduling=info,naga=error,wgpu_core=error,wgpu_hal=error \
RUST_BACKTRACE=1 \
cargo run --manifest-path cli/Cargo.toml -- run -c config/config.ron
