#!/bin/bash
set -e

while [[ $1 != '--' ]]; do
  perfargs+=("$1") # Collect arguments for perf before "--"
  shift
done
shift # Ignore "--"
profileargs=("$@") # What's left goes to profile

printf -v label '%s_' "${profileargs[@]}"

cargo build --release --features "profiling,unchecked" --bin profile
sudo perf record "${perfargs[@]}" --delay 400 --freq 99 --call-graph lbr -o "perf_${label}.data" ./target/release/profile "${profileargs[@]}" --delay 0.5
sudo chown $USER "perf_${label}.data"
perf script -i "perf_${label}.data" > "profile_${label}.perf"
perf report --call-graph -M intel -i "perf_${label}.data"
