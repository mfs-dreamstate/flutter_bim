#!/bin/bash
# Build Rust library for Android

set -e

echo "Building Rust library for Android..."

cd "$(dirname "$0")"

# Build for each Android architecture
echo "Building for arm64-v8a..."
cargo ndk -t arm64-v8a -o ../android/app/src/main/jniLibs build --release

echo "Building for armeabi-v7a..."
cargo ndk -t armeabi-v7a -o ../android/app/src/main/jniLibs build --release

echo "Building for x86_64..."
cargo ndk -t x86_64 -o ../android/app/src/main/jniLibs build --release

echo "Android build complete!"
echo "Libraries placed in: android/app/src/main/jniLibs/"
