#!/bin/bash
set -e

while [[ $1 != '--' ]]; do
  perfargs+=("$1") # Collect arguments for perf before "--"
  shift
done
shift # Ignore "--"
profileargs=("$@") # What's left goes to profile

cargo build --release --features "cli,profiling,unchecked" --bin impact
sudo perf stat "${perfargs[@]}" --delay 400 --detailed ./target/release/impact profile "${profileargs[@]}" --delay 0.5
