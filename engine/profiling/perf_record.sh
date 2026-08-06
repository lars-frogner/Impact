#!/bin/bash
set -e

while [[ $1 != '--' ]]; do
  perfargs+=("$1") # Collect arguments for perf before "--"
  shift
done
shift # Ignore "--"
benchmarkargs=("$@") # What's left goes to benchmark

printf -v label '%s_' "${benchmarkargs[@]}"

RUSTFLAGS="-C force-frame-pointers=yes" cargo build --release --features "cli,benchmark,unchecked" --bin impact
sudo perf record "${perfargs[@]}" --delay 400 --call-graph fp -o "perf_${label}.data" ./target/release/impact benchmark "${benchmarkargs[@]}" --delay 0.5
sudo chown $USER "perf_${label}.data"
perf script -i "perf_${label}.data" > "profile_${label}.perf"
perf report --call-graph -i "perf_${label}.data"
