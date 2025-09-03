#!/bin/bash

set -e

# Script to build the iOS library crate in the workspace
echo "Building Rust iOS library crate..."

# Install iOS targets if not already installed
rustup target add aarch64-apple-ios
rustup target add x86_64-apple-ios
rustup target add aarch64-apple-ios-sim

# Create output directories
mkdir -p target/universal/release

# Build the ios_lib crate for iOS device (ARM64)
echo "Building for iOS device (aarch64-apple-ios)..."
cargo build -p rust_ios_lib --target aarch64-apple-ios --release

# Build the ios_lib crate for iOS simulator (x86_64) 
echo "Building for iOS simulator (x86_64-apple-ios)..."
cargo build -p rust_ios_lib --target x86_64-apple-ios --release

# Build the ios_lib crate for iOS simulator (ARM64)
echo "Building for iOS simulator (aarch64-apple-ios-sim)..."
cargo build -p rust_ios_lib --target aarch64-apple-ios-sim --release

# Create universal library for simulators
echo "Creating universal library for simulators..."
lipo -create \
    target/x86_64-apple-ios/release/librust_ios_lib.a \
    target/aarch64-apple-ios-sim/release/librust_ios_lib.a \
    -output target/universal/release/librust_ios_lib.a

# Copy device library separately
echo "Copying device library..."
cp target/aarch64-apple-ios/release/librust_ios_lib.a target/universal/release/librust_ios_lib_device.a

# Copy the C header if it exists
if [ -f "crates/ios_lib/rust_ios_lib.h" ]; then
    echo "Copying C header file..."
    cp crates/ios_lib/rust_ios_lib.h target/universal/release/
fi

echo ""
echo "✅ Rust iOS library built successfully!"
echo "📦 Universal simulator library: target/universal/release/librust_ios_lib.a"
echo "📱 Device library: target/universal/release/librust_ios_lib_device.a"
echo ""
echo "To build the server separately, run:"
echo "cargo build -p coin-crab-server --release"