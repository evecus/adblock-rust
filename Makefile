# dns-filter Makefile
# Requires: Docker (for cross-compile), Node.js + npm (for frontend)

IMAGE     := ghcr.io/cross-rs/cross:latest
CARGO     := cargo
CROSS     := cross
VERSION   := $(shell grep '^version' crates/dns-server/Cargo.toml | head -1 | cut -d'"' -f2)
OUT       := dist

.PHONY: all build-amd64 build-arm64 build-convert frontend clean install help

## Default: build everything for both targets
all: frontend build-amd64 build-arm64 build-convert

## Build dns-filter daemon for linux/amd64
build-amd64:
	@echo "→ Building dns-filter (linux/amd64)..."
	@mkdir -p $(OUT)
	$(CROSS) build --release --bin dns-filter \
		--target x86_64-unknown-linux-musl \
		--manifest-path crates/dns-server/Cargo.toml
	cp target/x86_64-unknown-linux-musl/release/dns-filter \
		$(OUT)/dns-filter-linux-amd64
	@echo "✓ $(OUT)/dns-filter-linux-amd64"

## Build dns-filter daemon for linux/arm64
build-arm64:
	@echo "→ Building dns-filter (linux/arm64)..."
	@mkdir -p $(OUT)
	$(CROSS) build --release --bin dns-filter \
		--target aarch64-unknown-linux-musl \
		--manifest-path crates/dns-server/Cargo.toml
	cp target/aarch64-unknown-linux-musl/release/dns-filter \
		$(OUT)/dns-filter-linux-arm64
	@echo "✓ $(OUT)/dns-filter-linux-arm64"

## Build ars-convert tool for both targets
build-convert:
	@echo "→ Building ars-convert (amd64 + arm64)..."
	@mkdir -p $(OUT)
	$(CROSS) build --release --bin ars-convert \
		--target x86_64-unknown-linux-musl
	$(CROSS) build --release --bin ars-convert \
		--target aarch64-unknown-linux-musl
	cp target/x86_64-unknown-linux-musl/release/ars-convert  $(OUT)/ars-convert-linux-amd64
	cp target/aarch64-unknown-linux-musl/release/ars-convert $(OUT)/ars-convert-linux-arm64
	@echo "✓ ars-convert built"

## Build frontend and embed into web-api
frontend:
	@echo "→ Building frontend..."
	cd frontend && npm install && npm run build
	@echo "✓ Frontend built → crates/web-api/src/dist/"

## Build locally (native, debug)
dev:
	$(CARGO) build

## Run tests
test:
	$(CARGO) test

## Install locally (native release build)
install:
	$(CARGO) install --path crates/dns-server --bin dns-filter
	$(CARGO) install --path crates/ars-convert --bin ars-convert

## Convert a rule file and run locally for testing
run-dev:
	$(CARGO) run --bin dns-filter -- --config config.json

## Docker build image (bundles both binaries + frontend)
docker:
	docker build -t dns-filter:$(VERSION) .

## Package release archives
release: all
	@mkdir -p $(OUT)/release
	tar -czf $(OUT)/release/dns-filter-$(VERSION)-linux-amd64.tar.gz \
		-C $(OUT) dns-filter-linux-amd64 ars-convert-linux-amd64 \
		-C $(CURDIR) config.example.json README.md
	tar -czf $(OUT)/release/dns-filter-$(VERSION)-linux-arm64.tar.gz \
		-C $(OUT) dns-filter-linux-arm64 ars-convert-linux-arm64 \
		-C $(CURDIR) config.example.json README.md
	@echo "✓ Release archives in $(OUT)/release/"

clean:
	$(CARGO) clean
	rm -rf $(OUT)
	rm -rf frontend/node_modules frontend/dist
	rm -rf crates/web-api/src/dist

help:
	@grep -E '^## ' Makefile | sed 's/## /  /'
