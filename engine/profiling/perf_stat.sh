#!/bin/bash
set -e

while [[ $1 != '--' ]]; do
  perfargs+=("$1") # Collect arguments for perf before "--"
  shift
done
shift # Ignore "--"
benchmarkargs=("$@") # What's left goes to benchmark

cargo build --release --features "cli,benchmark,unchecked" --bin impact
sudo perf stat "${perfargs[@]}" --delay 400 --detailed ./target/release/impact benchmark "${benchmarkargs[@]}" --delay 0.5
