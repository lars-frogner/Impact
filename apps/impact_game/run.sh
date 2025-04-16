#!/bin/bash
set -e
roc build.roc
RUST_LOG=error,impact_game=debug cargo run --manifest-path cli/Cargo.toml --release -- run -c config/config.ron
