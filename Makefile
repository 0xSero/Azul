# Azul Browse - Build Automation
# Usage: make <target>

.PHONY: all build release install uninstall test lint fmt check clean help version bump-patch bump-minor bump-major tag

# Configuration
BINARY_NAME := azul
INSTALL_PATH := /usr/local/bin
VERSION := $(shell grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)

# Default target
all: build

# ============================================================================
# Development
# ============================================================================

## build: Build debug binary
build:
	cargo build

## release: Build optimized release binary
release:
	cargo build --release

## run: Run the application
run:
	cargo run

## run-release: Run the release build
run-release:
	cargo run --release

## test: Run all tests
test:
	cargo test --all-features

## lint: Run clippy linter
lint:
	cargo clippy --all-targets --all-features -- -D warnings

## fmt: Format code
fmt:
	cargo fmt

## fmt-check: Check code formatting
fmt-check:
	cargo fmt --all -- --check

## check: Run all checks (fmt, lint, test)
check: fmt-check lint test
	@echo "✓ All checks passed!"

## clean: Clean build artifacts
clean:
	cargo clean

# ============================================================================
# Installation
# ============================================================================

## install: Build and install to system (requires sudo)
install: release
	@echo "Installing $(BINARY_NAME) to $(INSTALL_PATH)..."
	sudo cp target/release/$(BINARY_NAME) $(INSTALL_PATH)/$(BINARY_NAME)
	@echo "✓ Installed $(BINARY_NAME) v$(VERSION)"

## install-user: Install to ~/.local/bin (no sudo)
install-user: release
	@mkdir -p ~/.local/bin
	cp target/release/$(BINARY_NAME) ~/.local/bin/$(BINARY_NAME)
	@echo "✓ Installed $(BINARY_NAME) v$(VERSION) to ~/.local/bin"
	@echo "  Make sure ~/.local/bin is in your PATH"

## uninstall: Remove from system
uninstall:
	sudo rm -f $(INSTALL_PATH)/$(BINARY_NAME)
	@echo "✓ Uninstalled $(BINARY_NAME)"

## cargo-install: Install via cargo
cargo-install:
	cargo install --path .

# ============================================================================
# Versioning & Release
# ============================================================================

## version: Show current version
version:
	@echo "$(VERSION)"

## bump-patch: Bump patch version (0.0.X -> 0.0.X+1)
bump-patch:
	@NEW_VERSION=$$(echo $(VERSION) | awk -F. '{print $$1"."$$2"."$$3+1}'); \
	sed -i 's/^version = "$(VERSION)"/version = "'$$NEW_VERSION'"/' Cargo.toml; \
	echo "Bumped version: $(VERSION) -> $$NEW_VERSION"

## bump-minor: Bump minor version (0.X.0 -> 0.X+1.0)
bump-minor:
	@NEW_VERSION=$$(echo $(VERSION) | awk -F. '{print $$1"."$$2+1".0"}'); \
	sed -i 's/^version = "$(VERSION)"/version = "'$$NEW_VERSION'"/' Cargo.toml; \
	echo "Bumped version: $(VERSION) -> $$NEW_VERSION"

## bump-major: Bump major version (X.0.0 -> X+1.0.0)
bump-major:
	@NEW_VERSION=$$(echo $(VERSION) | awk -F. '{print $$1+1".0.0"}'); \
	sed -i 's/^version = "$(VERSION)"/version = "'$$NEW_VERSION'"/' Cargo.toml; \
	echo "Bumped version: $(VERSION) -> $$NEW_VERSION"

## tag: Create a git tag for current version
tag:
	@echo "Creating tag v$(VERSION)..."
	git tag -a "v$(VERSION)" -m "Release v$(VERSION)"
	@echo "✓ Created tag v$(VERSION)"
	@echo "  Push with: git push origin v$(VERSION)"

## release-tag: Create and push release tag (triggers GitHub release)
release-tag: tag
	git push origin "v$(VERSION)"
	@echo "✓ Pushed tag v$(VERSION) - GitHub Actions will create the release"

# ============================================================================
# Help
# ============================================================================

## help: Show this help message
help:
	@echo "Azul Browse v$(VERSION) - Build Commands"
	@echo ""
	@echo "Usage: make <target>"
	@echo ""
	@echo "Development:"
	@grep -E '^## ' $(MAKEFILE_LIST) | grep -E '(build|release|run|test|lint|fmt|check|clean):' | \
		sed 's/## /  /' | sed 's/: /\t/'
	@echo ""
	@echo "Installation:"
	@grep -E '^## ' $(MAKEFILE_LIST) | grep -E '(install|uninstall|cargo):' | \
		sed 's/## /  /' | sed 's/: /\t/'
	@echo ""
	@echo "Versioning:"
	@grep -E '^## ' $(MAKEFILE_LIST) | grep -E '(version|bump|tag):' | \
		sed 's/## /  /' | sed 's/: /\t/'
