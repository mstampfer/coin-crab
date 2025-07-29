#!/bin/bash

set -e

# Install iOS targets if not already installed
rustup target add aarch64-apple-ios
rustup target add x86_64-apple-ios
rustup target add aarch64-apple-ios-sim

# Create output directories
mkdir -p target/universal/release

# Build for iOS device (ARM64)
cargo build --target aarch64-apple-ios --release

# Build for iOS simulator (x86_64)
cargo build --target x86_64-apple-ios --release

# Build for iOS simulator (ARM64)
cargo build --target aarch64-apple-ios-sim --release

# Create universal library for simulators
lipo -create \
    target/x86_64-apple-ios/release/librust_ios_lib.a \
    target/aarch64-apple-ios-sim/release/librust_ios_lib.a \
    -output target/universal/release/librust_ios_lib.a

# Copy device library separately
cp target/aarch64-apple-ios/release/librust_ios_lib.a target/universal/release/librust_ios_lib_device.a

echo "Rust library built successfully!"
echo "Universal simulator library: target/universal/release/librust_ios_lib.a"
echo "Device library: target/universal/release/librust_ios_lib_device.a"