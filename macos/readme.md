# Notecognito macOS Implementation

A native macOS menu bar application that displays translucent notecard windows via global hotkeys.

## Features

### Menu Bar Integration
- Lives in the menu bar (status bar) with no dock icon
- Minimal UI footprint
- Right-click menu for configuration and quit
- Native macOS look and feel

### Global Hotkeys
- Default: `⌘ Cmd+⇧ Shift+[1-9]` (customizable)
- Works across all applications and spaces
- Requires accessibility permissions
- Instant notecard display

### Translucent Notecards
- Adjustable opacity (20-100%)
- Click or Escape to dismiss
- Auto-hide timer support
- Smooth macOS window animations
- Dark background for readability

### Display Options
- Customizable position and size
- Multiple font families including SF Pro
- Adjustable font size (10-36pt)
- Rounded corners with shadow
- Multi-space support

## Prerequisites

- macOS 10.14 (Mojave) or later
- Xcode Command Line Tools
- Rust 1.70+ with cargo

## Building

1. **Install Rust:**
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Clone and build:**
   ```bash
   cd macos
   chmod +x build.sh
   ./build.sh
   ```

3. **The app bundle will be at:**
   ```
   target/release/Notecognito.app
   ```

## Installation

1. **Copy to Applications:**
   ```bash
   cp -r target/release/Notecognito.app /Applications/
   ```

2. **First Launch:**
    - Open from Applications or Launchpad
    - Grant accessibility permissions when prompted
    - Look for the icon in the menu bar

3. **Permissions:**
    - Go to System Preferences → Security & Privacy → Privacy → Accessibility
    - Add Notecognito if not already listed
    - Ensure the checkbox is checked

## Usage

### Menu Bar Icon
- Click for menu options:
    - **Configure**: Opens the Electron configuration UI
    - **About**: Shows version information
    - **Quit**: Exits the application

### Hotkeys
- Press `⌘+⇧+[1-9]` to display notecards
- Only notecards with content will appear
- Customize modifier keys in configuration

### Dismissing Notecards
- Click on the notecard
- Press Escape key
- Wait for auto-hide timer (if configured)

## Configuration

The app connects to the Notecognito core service for configuration. Ensure the core IPC server is running:

```bash
# In the core directory
cargo run --bin notecognito-ipc-server
```

Configuration is stored in:
```
~/Library/Application Support/notecognito/config.json
```

## Project Structure

```
macos/
├── Cargo.toml              # Project configuration
├── build.sh                # Build script
├── src/
│   ├── main.rs             # Application entry point
│   ├── app_delegate.rs     # NSApplication delegate
│   ├── hotkey.rs           # Global hotkey management
│   ├── ipc_client.rs       # Communication with core
│   ├── notecard_window.rs  # NSWindow-based overlays
│   └── platform_impl.rs    # macOS platform implementation
└── assets/
    ├── icon.icns           # macOS app icon
    └── icon.png            # Menu bar icon (16x16 @2x)
```

## Technical Details

### Menu Bar App
- Uses `LSUIElement = true` for no dock icon
- NSStatusItem for menu bar presence
- NSMenu for dropdown options
- Native macOS menu styling

### Hotkey Registration
- Core Graphics Event Taps for global hotkeys
- Requires accessibility permissions
- Runs in separate thread
- Modifier key combinations supported

### Window Management
- NSWindow with NSWindowStyleMask::Borderless
- NSWindowLevel::FloatingWindow for always-on-top
- Core Animation for smooth transitions
- Per-space window behavior

### Permissions
- Accessibility API for hotkeys
- No network access except localhost IPC
- Sandboxing compatible
- Code signing ready

## Troubleshooting

### "Notecognito needs accessibility permissions"
1. Open System Preferences → Security & Privacy
2. Click Privacy tab → Accessibility
3. Click the lock to make changes
4. Add Notecognito or ensure it's checked
5. Restart the app

### Hotkeys not working
- Verify accessibility permissions are granted
- Check if another app uses the same hotkeys
- Try different modifier combinations
- Check Console.app for error messages

### Menu bar icon not appearing
- Check Activity Monitor for Notecognito process
- Try `killall Notecognito` and restart
- Check for crashes in Console.app
- Verify the app bundle structure

### "Failed to connect to core service"
- Ensure the core IPC server is running
- Check firewall settings for localhost:7855
- The app will run in standalone mode if unavailable

## Security Considerations

- Event Taps require user consent
- No keylogging - only registered hotkeys captured
- All data stored in user's Library folder
- IPC restricted to localhost only
- Supports macOS Gatekeeper and notarization

## Development

### Debug Build
```bash
cargo build
RUST_LOG=debug target/debug/Notecognito
```

### Console Logs
```bash
# View logs in real-time
log stream --predicate 'process == "Notecognito"'
```

### Testing Permissions
The app will check and request permissions on startup. For testing, you can reset permissions:
```bash
tccutil reset Accessibility com.notecognito.macos
```

## Distribution

1. **Code Signing:**
   ```bash
   codesign --deep --force --verify --verbose --sign "Developer ID Application: Your Name" Notecognito.app
   ```

2. **Notarization:**
   ```bash
   xcrun altool --notarize-app --primary-bundle-id "com.notecognito.macos" \
     --username "your@email.com" --password "@keychain:notarize" \
     --file Notecognito.app.zip
   ```

3. **DMG Creation:**
   ```bash
   create-dmg --volname "Notecognito" --window-size 600 400 \
     --icon "Notecognito.app" 175 200 --app-drop-link 425 200 \
     "Notecognito.dmg" "Notecognito.app"
   ```

## Known Issues

- Hotkeys may not work in some full-screen games
- Some virtualization software may interfere with Event Taps
- macOS Ventura+ may show additional permission prompts

## Future Enhancements

- Touch Bar support
- Shortcuts app integration
- Widget for Notification Center
- Universal Binary (Apple Silicon + Intel)