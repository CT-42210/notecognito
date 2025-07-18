#!/bin/bash

# Build all Notecognito components

echo "=================================="
echo "Building Notecognito Components"
echo "=================================="
echo ""

# Detect OS
OS="unknown"
if [[ "$OSTYPE" == "darwin"* ]]; then
    OS="macos"
elif [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "cygwin" ]] || [[ "$OSTYPE" == "win32" ]]; then
    OS="windows"
elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
    OS="linux"
fi

echo "Detected OS: $OS"
echo ""

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check prerequisites
echo "Checking prerequisites..."

if ! command_exists cargo; then
    echo "ERROR: Rust/Cargo not found. Please install from https://rustup.rs/"
    exit 1
fi

if ! command_exists npm; then
    echo "ERROR: Node.js/npm not found. Please install from https://nodejs.org/"
    exit 1
fi

echo "✓ All prerequisites found"
echo ""

# Build Core Library
echo "=================================="
echo "Building Core Library"
echo "=================================="
cd core
if [[ -f "build.sh" ]]; then
    chmod +x build.sh
    ./build.sh
else
    cargo build --release --features ffi
    cargo build --release --bin notecognito-ipc-server
fi
cd ..
echo ""

# Build Electron Configuration UI
echo "=================================="
echo "Building Electron Configuration UI"
echo "=================================="
cd app
npm install
if [[ "$OS" == "macos" ]]; then
    npm run build -- --mac
elif [[ "$OS" == "windows" ]]; then
    npm run build -- --win
else
    npm run build -- --linux
fi
cd ..
echo ""

# Build Platform-Specific Application
if [[ "$OS" == "macos" ]]; then
    echo "=================================="
    echo "Building macOS Application"
    echo "=================================="
    cd macos
    if [[ -f "build.sh" ]]; then
        chmod +x build.sh
        ./build.sh
    else
        cargo build --release
    fi
    cd ..
elif [[ "$OS" == "windows" ]]; then
    echo "=================================="
    echo "Building Windows Application"
    echo "=================================="
    cd windows
    if [[ -f "build.bat" ]]; then
        ./build.bat
    else
        cargo build --release
    fi
    cd ..
else
    echo "Platform-specific build not available for Linux yet"
fi

echo ""
echo "=================================="
echo "Build Summary"
echo "=================================="
echo ""
echo "✓ Core library built"
echo "  - Library: core/target/release/"
echo "  - IPC Server: core/target/release/notecognito-ipc-server"
echo ""
echo "✓ Configuration UI built"
echo "  - App bundles: app/dist/"
echo ""

if [[ "$OS" == "macos" ]]; then
    echo "✓ macOS application built"
    echo "  - App bundle: macos/target/release/Notecognito.app"
elif [[ "$OS" == "windows" ]]; then
    echo "✓ Windows application built"
    echo "  - Executable: windows/target/release/notecognito.exe"
fi

echo ""
echo "=================================="
echo "Next Steps"
echo "=================================="
echo ""
echo "1. Start the IPC server:"
echo "   cd core && cargo run --bin notecognito-ipc-server"
echo ""
echo "2. Run the platform application:"
if [[ "$OS" == "macos" ]]; then
    echo "   open macos/target/release/Notecognito.app"
elif [[ "$OS" == "windows" ]]; then
    echo "   windows\\target\\release\\notecognito.exe"
fi
echo ""
echo "3. Configure notecards using the UI"
echo ""
echo "For detailed instructions, see the README in each component directory."