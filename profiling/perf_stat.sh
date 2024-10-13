#!/bin/bash
set -e

while [[ $1 != '--' ]]; do
  perfargs+=("$1") # Collect arguments for perf before "--"
  shift
done
shift # Ignore "--"
profileargs=("$@") # What's left goes to profile

cargo build --release --features profiling --bin profile
sudo perf stat "${perfargs[@]}" --delay 400 --detailed ./target/release/profile "${profileargs[@]}" --delay 0.5
