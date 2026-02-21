#!/bin/bash
# Build Rust native libraries for all platforms
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
RUST_DIR="$PROJECT_DIR/rust"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log() { echo -e "${GREEN}[BUILD]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Check prerequisites
command -v cargo >/dev/null 2>&1 || { error "cargo not found. Install Rust: https://rustup.rs"; exit 1; }

build_android() {
    log "Building for Android..."
    command -v cargo-ndk >/dev/null 2>&1 || {
        warn "cargo-ndk not found, installing..."
        cargo install cargo-ndk
    }

    cd "$RUST_DIR"

    log "  Building arm64-v8a..."
    cargo ndk -t arm64-v8a -o "$PROJECT_DIR/android/src/main/jniLibs" build --release

    log "  Building x86_64..."
    cargo ndk -t x86_64 -o "$PROJECT_DIR/android/src/main/jniLibs" build --release

    log "Android build complete!"
}

build_ios() {
    log "Building for iOS..."

    if [[ "$(uname)" != "Darwin" ]]; then
        error "iOS builds require macOS"
        return 1
    fi

    cd "$RUST_DIR"

    rustup target add aarch64-apple-ios 2>/dev/null || true
    rustup target add aarch64-apple-ios-sim 2>/dev/null || true

    log "  Building iOS device (ARM64)..."
    cargo build --release --target aarch64-apple-ios

    log "  Building iOS simulator (ARM64)..."
    cargo build --release --target aarch64-apple-ios-sim

    # Copy libraries
    mkdir -p "$PROJECT_DIR/ios/Runner"
    cp "$RUST_DIR/target/aarch64-apple-ios/release/librust.a" "$PROJECT_DIR/ios/Runner/librust_device.a"
    cp "$RUST_DIR/target/aarch64-apple-ios-sim/release/librust.a" "$PROJECT_DIR/ios/Runner/librust_sim.a"

    log "iOS build complete!"
}

run_tests() {
    log "Running Rust tests..."
    cd "$RUST_DIR"
    cargo test
    log "All tests passed!"
}

# Parse arguments
case "${1:-all}" in
    android) build_android ;;
    ios) build_ios ;;
    test) run_tests ;;
    all)
        run_tests
        build_android
        if [[ "$(uname)" == "Darwin" ]]; then
            build_ios
        else
            warn "Skipping iOS build (not on macOS)"
        fi
        ;;
    *) echo "Usage: $0 {android|ios|test|all}"; exit 1 ;;
esac

log "Done!"
