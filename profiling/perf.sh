#!/bin/bash
RUSTFLAGS="-g" cargo build --release --bin profile
sudo perf record -g ./target/release/profile "$@"
sudo chown $USER perf.data
perf script > profile.perf