#!/bin/bash
set -e
mkdir -p lib roc_platform/lib
roc build.roc
RUST_LOG=debug,impact::thread=info,impact::thread=info,impact::scheduling=info,naga=error,wgpu_core=error,wgpu_hal=error \
cargo run --manifest-path cli/Cargo.toml --release -- run -c config/config.ron
