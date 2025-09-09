# Basic App Makefile

# Configuration
APP_NAME := basic_app
CONFIG_FILE := config/config.ron
DEFAULT_LOG_LEVEL := info

# Common environment variables
define RUST_LOG_QUIET
error,calloop=error,naga=error,wgpu_core=error,wgpu_hal=error
endef

define RUST_LOG_NORMAL
$(or $(LOG_LEVEL),$(DEFAULT_LOG_LEVEL)),calloop=error,naga=error,wgpu_core=error,wgpu_hal=error
endef

.PHONY: all clean help
.PHONY: init-build build build-debug build-debug-cl build-debug-asan
.PHONY: run run-debug run-roc-debug run-debug-asan run-debug-cl
.PHONY: profile-perf profile-tracy profile-heaptrack profile-dhat
.PHONY: fuzz
.PHONY: generate-roc clean-roc

# Default target
all: build

# Build the roc build script if build.roc changes
build_script: build.roc
	roc build --optimize --output build_script build.roc

# Convenience target for init-build
init-build: build_script

# Build targets with dependencies
dist/release/$(APP_NAME): build_script
	OUTPUT_DIR=./dist/release ./build_script

dist/debug/$(APP_NAME): build_script
	OUTPUT_DIR=./dist/debug ROC_DEBUG=1 RUST_DEBUG=1 ./build_script

dist/roc_debug/$(APP_NAME): build_script
	OUTPUT_DIR=./dist/roc_debug ROC_DEBUG=1 ./build_script

dist/debug_asan/$(APP_NAME): build_script
	OUTPUT_DIR=./dist/debug_asan ROC_DEBUG=1 RUST_DEBUG=1 ASAN=1 ./build_script

dist/debug_cranelift/$(APP_NAME): build_script
	OUTPUT_DIR=./dist/debug_cranelift CRANELIFT=1 ROC_DEBUG=1 RUST_DEBUG=1 ./build_script

dist/perf/$(APP_NAME): build_script
	OUTPUT_DIR=./dist/perf PROFILING=1 ./build_script

dist/tracy/$(APP_NAME): build_script
	OUTPUT_DIR=./dist/tracy PROFILING=1 TRACY=1 ./build_script

dist/heaptrack/$(APP_NAME): build_script
	OUTPUT_DIR=./dist/heaptrack PROFILING=1 ./build_script

dist/dhat/$(APP_NAME): build_script
	OUTPUT_DIR=./dist/dhat PROFILING=1 VALGRIND=1 ./build_script

dist/fuzz/$(APP_NAME): build_script
	OUTPUT_DIR=./dist/fuzz RUST_DEBUG=1 ASAN=1 FUZZING=1 ./build_script

# Building
build: dist/release/$(APP_NAME)

build-debug: dist/debug/$(APP_NAME)

build-roc-debug: dist/roc_debug/$(APP_NAME)

build-debug-cl: dist/debug_cranelift/$(APP_NAME)

build-debug-asan: dist/debug_asan/$(APP_NAME)

# Running
run: dist/release/$(APP_NAME)
	RUST_LOG="$(RUST_LOG_NORMAL)" ./dist/release/$(APP_NAME) run -c $(CONFIG_FILE)

run-debug: dist/debug/$(APP_NAME)
	RUST_LOG="$(RUST_LOG_NORMAL)" RUST_BACKTRACE=1 ./dist/debug/$(APP_NAME) run -c $(CONFIG_FILE)

run-roc-debug: dist/roc_debug/$(APP_NAME)
	RUST_LOG="$(RUST_LOG_NORMAL)" RUST_BACKTRACE=1 ./dist/roc_debug/$(APP_NAME) run -c $(CONFIG_FILE)

run-debug-cl: dist/debug_cranelift/$(APP_NAME)
	RUST_LOG="$(RUST_LOG_NORMAL)" RUST_BACKTRACE=1 ./dist/debug_cranelift/$(APP_NAME) run -c $(CONFIG_FILE)

run-debug-asan: dist/debug_asan/$(APP_NAME)
	LD_PRELOAD=/usr/lib/gcc/x86_64-linux-gnu/13/libasan.so \
	ASAN_OPTIONS="detect_leaks=1:halt_on_error=1:allocator_may_return_null=1:detect_deadlocks=1:detect_stack_use_after_return=1" \
	RUST_LOG="$(RUST_LOG_NORMAL)" \
	RUST_BACKTRACE=1 \
	LC_ALL=en_US.UTF-8 \
	./dist/debug_asan/$(APP_NAME) run -c $(CONFIG_FILE)

# Profiling
profile-perf: dist/perf/$(APP_NAME)
	sudo env RUST_LOG="$(RUST_LOG_QUIET)" \
		perf record --delay 5000 --freq 1000 --call-graph dwarf ./dist/perf/$(APP_NAME) run -c $(CONFIG_FILE)
	sudo chown $(USER) "perf.data"
	perf script -i "perf.data" > "profile.perf"
	perf report --call-graph -M intel -i "perf.data"

profile-tracy: dist/tracy/$(APP_NAME)
	RUST_LOG="$(RUST_LOG_NORMAL)" RUST_BACKTRACE=1 ./dist/tracy/$(APP_NAME) run -c $(CONFIG_FILE)

profile-heaptrack: dist/heaptrack/$(APP_NAME)
	heaptrack ./dist/heaptrack/$(APP_NAME) run -c $(CONFIG_FILE)

profile-dhat: dist/dhat/$(APP_NAME)
	RUST_LOG="$(RUST_LOG_NORMAL)" RUST_BACKTRACE=1 valgrind --tool=dhat --num-callers=500 ./dist/dhat/$(APP_NAME) run -c $(CONFIG_FILE)

# Fuzzing
fuzz: dist/fuzz/$(APP_NAME)
	LD_PRELOAD=/usr/lib/gcc/x86_64-linux-gnu/13/libasan.so \
	ASAN_OPTIONS="detect_leaks=1:halt_on_error=1:allocator_may_return_null=1:detect_deadlocks=1:detect_stack_use_after_return=1:strict_string_checks=1" \
	RUST_BACKTRACE=1 \
	./dist/fuzz/$(APP_NAME) fuzz $(ARGS)

# Roc codegen
generate-roc:
	cargo run --manifest-path tools/generate_roc/Cargo.toml --release -- generate-modules -t roc_platform/api -p pf -v

clean-roc:
	cargo run --manifest-path tools/generate_roc/Cargo.toml --release -- clean -v -r -t roc_platform/api

# Cleanup
clean:
	rm -rf build_script lib/ dist/

clean-profiling:
	rm -rf perf.data profile.perf dhat.out.* heaptrack.*.zst

# Help
help:
	@echo "Basic App Build System"
	@echo ""
	@echo "Building:"
	@echo "  init-build        - Build the Roc build script"
	@echo "  build             - Build app crate and Roc script in release mode"
	@echo "  build-debug       - Build app crate and Roc script in debug mode"
	@echo "  build-roc-debug   - Build app crate in release mode and Roc script in debug mode"
	@echo "  build-debug-cl    - Build app crate and Roc script in debug mode, using Cranelift as Rust codegen backend"
	@echo "  build-debug-asan  - Build app crate and Roc script in debug mode with AddressSanitizer enabled"
	@echo ""
	@echo "Running:"
	@echo "  run               - Build app crate and Roc script in release mode and run"
	@echo "  run-debug         - Build app crate and Roc script in debug mode and run"
	@echo "  run-roc-debug     - Build app crate in release mode and Roc script in debug mode and run"
	@echo "  run-debug-cl      - Build app crate and Roc script in debug mode, using Cranelift as Rust codegen backend, and run"
	@echo "  run-debug-asan    - Build app crate and Roc script in debug mode with AddressSanitizer enabled and run"
	@echo ""
	@echo "Profiling:"
	@echo "  profile-perf      - Build and run with perf profiling"
	@echo "  profile-tracy     - Build and run with Tracy profiling"
	@echo "  profile-heaptrack - Build and run with heaptrack profiling"
	@echo "  profile-dhat      - Build and run with DHAT profiling"
	@echo ""
	@echo "Fuzzing:"
	@echo "  fuzz              - Run a fuzz target (use ARGS= to pass arguments)"
	@echo ""
	@echo "Roc codegen:"
	@echo "  generate-roc      - Generate Roc code for platform API"
	@echo "  clean-roc         - Clean generated platform API Roc code"
	@echo ""
	@echo "Cleanup:"
	@echo "  clean             - Remove build artifacts"
	@echo "  clean-profiling   - Remove profiling artifacts"
	@echo ""
	@echo "Variables:"
	@echo "  LOG_LEVEL         - Set log level (default: info)"
	@echo "  Example: make run LOG_LEVEL=debug"
