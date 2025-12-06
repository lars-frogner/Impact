#!/bin/bash
set -e

cargo build --release --features "cli,benchmark,unchecked" --bin impact
heaptrack ./target/release/impact benchmark "$@"
