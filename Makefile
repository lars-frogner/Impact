ENGINE := engine
APPS   := apps/basic_app apps/impact_game apps/snapshot_tester apps/voxel_generator

ENGINE_MANIFEST := $(ENGINE)/Cargo.toml

APP_MANIFESTS := $(foreach app,$(APPS),$(app)/Cargo.toml)

APP_SUB_MANIFESTS := $(foreach app,$(APPS),\
    $(app)/cli/Cargo.toml \
    $(app)/roc_platform/Cargo.toml \
    $(app)/tools/generate_roc/Cargo.toml)

FUZZ_MANIFESTS := \
    engine/crates/impact_intersection/fuzz/Cargo.toml \
    engine/crates/impact_voxel/fuzz/Cargo.toml

EXTRA_MANIFESTS := \
    roc_platform/core/Cargo.toml \
    roc_integration/Cargo.toml \
    interop/dynamic_lib/Cargo.toml \
    interop/hashing/Cargo.toml \
    interop/log/Cargo.toml \
    tools/asset_fetcher/Cargo.toml

ALL_MANIFESTS := \
    $(ENGINE_MANIFEST) \
    $(APP_MANIFESTS) \
    $(APP_SUB_MANIFESTS) \
    $(EXTRA_MANIFESTS) \
    $(FUZZ_MANIFESTS)

DENY_MANIFESTS := \
    $(ENGINE_MANIFEST) \
    $(APP_MANIFESTS) \
    roc_integration/Cargo.toml \
    tools/asset_fetcher/Cargo.toml

.DEFAULT_GOAL := help

.PHONY: fmt clippy test deny udeps update clean generate-roc help

fmt:
	@for manifest in $(ALL_MANIFESTS); do \
		echo "=== fmt: $$manifest ==="; \
		cargo fmt --manifest-path $$manifest --all; \
	done

clippy:
	@for manifest in $(ALL_MANIFESTS); do \
		echo "=== clippy: $$manifest ==="; \
		cargo clippy --quiet --manifest-path $$manifest --workspace --all-targets --all-features; \
	done

test:
	@for manifest in $(ALL_MANIFESTS); do \
		echo "=== test: $$manifest ==="; \
		RUSTFLAGS="-Awarnings" cargo test --quiet --manifest-path $$manifest --workspace --all-features; \
	done

deny:
	@for manifest in $(DENY_MANIFESTS); do \
		echo "=== deny: $$manifest ==="; \
		cargo deny --manifest-path $$manifest --workspace --all-features check || true; \
	done

udeps:
	@for manifest in $(ALL_MANIFESTS); do \
		echo "=== udeps: $$manifest ==="; \
		RUSTFLAGS="-Awarnings" cargo +nightly udeps --quiet --manifest-path $$manifest --workspace --all-targets || true; \
	done

update:
	cargo update --manifest-path $(ENGINE_MANIFEST)
	@for app in $(APPS); do \
		$(MAKE) -C $$app cargo-update; \
	done
	@for manifest in $(EXTRA_MANIFESTS) $(FUZZ_MANIFESTS); do \
		cargo update --manifest-path $$manifest; \
	done

clean:
	cargo clean --manifest-path $(ENGINE_MANIFEST)
	@for app in $(APPS); do \
		$(MAKE) -C $$app cargo-clean clean; \
	done
	@for manifest in $(EXTRA_MANIFESTS); do \
		cargo clean --manifest-path $$manifest; \
	done
	@for manifest in $(FUZZ_MANIFESTS); do \
		cargo clean --manifest-path $$manifest; \
	done

generate-roc:
	@for app in $(APPS); do \
		$(MAKE) -C $$app generate-roc; \
	done

help:
	@echo "Top-level Makefile"
	@echo ""
	@echo "Targets:"
	@echo "  fmt           - Format all Rust code with cargo fmt"
	@echo "  clippy        - Lint all Rust code with cargo clippy"
	@echo "  test          - Run tests across all workspaces"
	@echo "  deny          - Check dependencies with cargo-deny"
	@echo "  udeps         - Check for unused dependencies (nightly + cargo-udeps)"
	@echo "  update        - Update all Cargo.lock files"
	@echo "  clean         - Remove all Cargo build artifacts"
	@echo "  generate-roc  - Generate Roc code for all apps"
