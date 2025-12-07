#!/bin/bash
set -e

if [[ $# -lt 1 ]]; then
    echo "Usage: $0 <collector> -- <benchmark args>"
    exit 1
fi

collector="$1"
shift

# Split args: everything after -- goes to the benchmark
if [[ $1 == "--" ]]; then
    shift
    bench_args=("$@")
else
    echo "Error: expected -- after collector name"
    exit 1
fi

# Build the binary
cargo build --release --features "cli,benchmark,unchecked" --bin impact

# Label based on collector + benchmark args
label=$(printf "%s_" "${collector}" "${bench_args[@]}" | tr -c 'A-Za-z0-9_' '_')
result_dir="vtune_${label}"

# Delete old result directory
rm -rf "${result_dir}"

# Map friendly collector aliases
case "$collector" in
    hotspots|hot)
        vtune_collector="hotspots"
        ;;
    uarch|uarch-exploration|micro)
        vtune_collector="uarch-exploration"
        ;;
    mem|memory|memory-access)
        vtune_collector="memory-access"
        ;;
    hwevents|hw|hw-events)
        vtune_collector="hw-events"
        ;;
    *)
        echo "Unknown collector: $collector"
        echo "Valid: hotspots | uarch | mem | hwevents"
        exit 1
        ;;
esac

# Run VTune collection
vtune -collect "${vtune_collector}" \
      -result-dir "${result_dir}" \
      -finalization-mode full \
      -- ./target/release/impact benchmark "${bench_args[@]}"

# Launch GUI
vtune-gui "${result_dir}"
