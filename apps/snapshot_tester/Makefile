# Snapshot Tester Makefile

# Configuration
APP_NAME := snapshot_tester
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
.PHONY: generate-roc clean-roc

# Default target
all: build

# Build the roc build script if build.roc changes
build_script: build.roc
	roc build --optimize --output build_script build.roc

# Convenience target for init-build
init-build: build_script

# Building
build: build_script
	OUTPUT_DIR=./dist/release ./build_script

build-debug: build_script
	OUTPUT_DIR=./dist/debug ROC_DEBUG=1 RUST_DEBUG=1 ./build_script

build-roc-debug: build_script
	OUTPUT_DIR=./dist/roc_debug ROC_DEBUG=1 ./build_script

build-debug-cl: build_script
	OUTPUT_DIR=./dist/debug_cranelift CRANELIFT=1 ROC_DEBUG=1 RUST_DEBUG=1 ./build_script

build-debug-asan: build_script
	OUTPUT_DIR=./dist/debug_asan ROC_DEBUG=1 RUST_DEBUG=1 ASAN=1 ./build_script

# Running
run: build
	RUST_LOG="$(RUST_LOG_NORMAL)" ./dist/release/$(APP_NAME) run -c $(CONFIG_FILE)

run-debug: build-debug
	RUST_LOG="$(RUST_LOG_NORMAL)" RUST_BACKTRACE=1 ./dist/debug/$(APP_NAME) run -c $(CONFIG_FILE)

run-roc-debug: build-roc-debug
	RUST_LOG="$(RUST_LOG_NORMAL)" RUST_BACKTRACE=1 ./dist/roc_debug/$(APP_NAME) run -c $(CONFIG_FILE)

run-debug-cl: build-debug-cl
	RUST_LOG="$(RUST_LOG_NORMAL)" RUST_BACKTRACE=1 ./dist/debug_cranelift/$(APP_NAME) run -c $(CONFIG_FILE)

run-debug-asan: build-debug-asan
	LD_PRELOAD=/usr/lib/gcc/x86_64-linux-gnu/13/libasan.so \
	ASAN_OPTIONS="detect_leaks=1:halt_on_error=1:allocator_may_return_null=1:detect_deadlocks=1:detect_stack_use_after_return=1" \
	RUST_LOG="$(RUST_LOG_NORMAL)" \
	RUST_BACKTRACE=1 \
	LC_ALL=en_US.UTF-8 \
	./dist/debug_asan/$(APP_NAME) run -c $(CONFIG_FILE)

# Roc codegen
generate-roc:
	cargo run --manifest-path tools/generate_roc/Cargo.toml --release -- generate-modules -t roc_platform/api -p pf -v

clean-roc:
	cargo run --manifest-path tools/generate_roc/Cargo.toml --release -- clean -v -r -t roc_platform/api

# Cleanup
clean:
	rm -rf build_script lib/ dist/ roc_platform/build roc_platform/lib/

# Cargo

cargo-clean:
	cargo clean
	cargo clean --manifest-path cli/Cargo.toml
	cargo clean --manifest-path roc_platform/Cargo.toml
	cargo clean --manifest-path tools/generate_roc/Cargo.toml

cargo-update:
	cargo update
	cargo update --manifest-path cli/Cargo.toml
	cargo update --manifest-path roc_platform/Cargo.toml
	cargo update --manifest-path tools/generate_roc/Cargo.toml

# Help
help:
	@echo "Snapshot Tester Build System"
	@echo ""
	@echo "Building:"
	@echo "  init-build       - Build the Roc build script"
	@echo "  build            - Build app crate and Roc script in release mode"
	@echo "  build-debug      - Build app crate and Roc script in debug mode"
	@echo "  build-roc-debug  - Build app crate in release mode and Roc script in debug mode"
	@echo "  build-debug-cl   - Build app crate and Roc script in debug mode, using Cranelift as Rust codegen backend"
	@echo "  build-debug-asan - Build app crate and Roc script in debug mode with AddressSanitizer enabled"
	@echo ""
	@echo "Running:"
	@echo "  run              - Build app crate and Roc script in release mode and run"
	@echo "  run-debug        - Build app crate and Roc script in debug mode and run"
	@echo "  run-roc-debug    - Build app crate in release mode and Roc script in debug mode and run"
	@echo "  run-debug-cl     - Build app crate and Roc script in debug mode, using Cranelift as Rust codegen backend, and run"
	@echo "  run-debug-asan   - Build app crate and Roc script in debug mode with AddressSanitizer enabled and run"
	@echo ""
	@echo "Roc codegen:"
	@echo "  generate-roc     - Generate Roc code for platform API"
	@echo "  clean-roc        - Clean generated platform API Roc code"
	@echo ""
	@echo "Cleanup:"
	@echo "  clean            - Remove build artifacts"
	@echo ""
	@echo "Cargo:"
	@echo "  cargo-clean      - Run `cargo clean` for all crates"
	@echo "  cargo-update     - Run `cargo update` for all crates"
	@echo ""
	@echo "Variables:"
	@echo "  LOG_LEVEL        - Set log level (default: info)"
	@echo "  Example: make run LOG_LEVEL=debug"
