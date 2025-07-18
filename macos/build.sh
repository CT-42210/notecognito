#!/bin/bash

# Build script for Notecognito macOS

echo "Building Notecognito for macOS..."

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "Error: Cargo not found. Please install Rust from https://rustup.rs/"
    exit 1
fi

# Create assets directory if it doesn't exist
if [ ! -d "assets" ]; then
    echo "Creating assets directory..."
    mkdir -p assets
fi

# Check for icon
if [ ! -f "assets/icon.icns" ]; then
    echo "Warning: assets/icon.icns not found"
    echo "Creating placeholder icon.icns from icon.png..."
    
    if [ -f "assets/icon.png" ]; then
        # Create iconset directory
        mkdir -p assets/icon.iconset
        
        # Generate different sizes
        sips -z 16 16     assets/icon.png --out assets/icon.iconset/icon_16x16.png
        sips -z 32 32     assets/icon.png --out assets/icon.iconset/icon_16x16@2x.png
        sips -z 32 32     assets/icon.png --out assets/icon.iconset/icon_32x32.png
        sips -z 64 64     assets/icon.png --out assets/icon.iconset/icon_32x32@2x.png
        sips -z 128 128   assets/icon.png --out assets/icon.iconset/icon_128x128.png
        sips -z 256 256   assets/icon.png --out assets/icon.iconset/icon_128x128@2x.png
        sips -z 256 256   assets/icon.png --out assets/icon.iconset/icon_256x256.png
        sips -z 512 512   assets/icon.png --out assets/icon.iconset/icon_256x256@2x.png
        sips -z 512 512   assets/icon.png --out assets/icon.iconset/icon_512x512.png
        sips -z 1024 1024 assets/icon.png --out assets/icon.iconset/icon_512x512@2x.png
        
        # Convert to icns
        iconutil -c icns assets/icon.iconset -o assets/icon.icns
        
        # Clean up
        rm -rf assets/icon.iconset
    else
        echo "Please add icon.png to the assets directory"
    fi
fi

# Build in release mode
echo "Building release version..."
cargo build --release

if [ $? -ne 0 ]; then
    echo "Error: Build failed!"
    exit 1
fi

echo ""
echo "Build successful!"
echo ""

# Create app bundle
APP_NAME="Notecognito"
BUNDLE_NAME="$APP_NAME.app"
BUNDLE_PATH="target/release/$BUNDLE_NAME"

echo "Creating app bundle..."

# Remove old bundle if exists
rm -rf "$BUNDLE_PATH"

# Create bundle structure
mkdir -p "$BUNDLE_PATH/Contents/MacOS"
mkdir -p "$BUNDLE_PATH/Contents/Resources"

# Copy executable
cp "target/release/Notecognito" "$BUNDLE_PATH/Contents/MacOS/"

# Copy icon
if [ -f "assets/icon.icns" ]; then
    cp "assets/icon.icns" "$BUNDLE_PATH/Contents/Resources/"
fi

# Copy icon.png for menu bar if exists
if [ -f "assets/icon.png" ]; then
    cp "assets/icon.png" "$BUNDLE_PATH/Contents/Resources/"
fi

# Create Info.plist
cat > "$BUNDLE_PATH/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>Notecognito</string>
    <key>CFBundleIconFile</key>
    <string>icon</string>
    <key>CFBundleIdentifier</key>
    <string>com.notecognito.macos</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>Notecognito</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.14</string>
    <key>LSUIElement</key>
    <true/>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSHumanReadableCopyright</key>
    <string>Copyright © 2024</string>
</dict>
</plist>
EOF

echo "App bundle created at: $BUNDLE_PATH"
echo ""

# Check if the IPC server is running
if ! nc -z localhost 7855 2>/dev/null; then
    echo "Note: The Notecognito core service is not running."
    echo "To start it, run in the core directory:"
    echo "  cargo run --bin notecognito-ipc-server"
    echo ""
fi

# Ask if user wants to run
read -p "Do you want to run Notecognito now? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo "Starting Notecognito..."
    open "$BUNDLE_PATH"
fi

echo ""
echo "To install:"
echo "  1. Copy $BUNDLE_PATH to /Applications"
echo "  2. Grant accessibility permissions when prompted"
echo "  3. Run from Applications or Launchpad"
echo ""