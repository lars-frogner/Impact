#!/bin/bash
cargo run --manifest-path tools/generate_roc/Cargo.toml --release -- clean -v -r -t roc_platform/api
