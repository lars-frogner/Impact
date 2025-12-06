#!/bin/bash
set -e

cargo build --release --features "cli,benchmark,unchecked" --bin impact
samply record ./target/release/impact benchmark "$@"
