#!/bin/bash

# Build script for Notecognito Core

echo "Building Notecognito Core Library..."

# Build the library in release mode
cargo build --release

# Build with FFI support
echo "Building with FFI support..."
cargo build --release --features ffi

# Generate C header using cbindgen (if installed)
if command -v cbindgen &> /dev/null; then
    echo "Generating C header..."
    cbindgen --config cbindgen.toml --crate notecognito-core --output notecognito_generated.h
else
    echo "cbindgen not found, skipping header generation"
fi

# Build the IPC server
echo "Building IPC server..."
cargo build --release --bin notecognito-ipc-server

# Run tests
echo "Running tests..."
cargo test

echo "Build complete!"

# Display build artifacts
echo ""
echo "Build artifacts:"
echo "  Library: target/release/libnotecognito_core.{so,dylib,dll}"
echo "  IPC Server: target/release/notecognito-ipc-server"

# Platform-specific notes
if [[ "$OSTYPE" == "darwin"* ]]; then
    echo ""
    echo "macOS: You can create a universal binary with:"
    echo "  cargo build --release --target x86_64-apple-darwin"
    echo "  cargo build --release --target aarch64-apple-darwin"
    echo "  lipo -create target/{x86_64,aarch64}-apple-darwin/release/libnotecognito_core.dylib -output libnotecognito_core_universal.dylib"
fi