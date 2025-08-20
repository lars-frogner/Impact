#!/bin/bash
set -e

OUTPUT_DIR=./dist/release
LOG_LEVEL=${1:-info}

OUTPUT_DIR=$OUTPUT_DIR \
RUST_ALLOCATOR=jemalloc \
roc build.roc

RUST_LOG="$LOG_LEVEL,calloop=error,naga=error,wgpu_core=error,wgpu_hal=error" \
_RJEM_MALLOC_CONF=prof:true,prof_active:true,lg_prof_sample:0,prof_final:true,prof_leak:true,prof_accum:true \
$OUTPUT_DIR/basic_app run -c config/config.ron
