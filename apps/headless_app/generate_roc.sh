#!/bin/bash
cargo run --manifest-path tools/generate_roc/Cargo.toml --release -- generate-modules -t roc_platform/api -p pf -v
