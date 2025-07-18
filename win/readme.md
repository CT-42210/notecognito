# Notecognito Windows Implementation

A Windows system tray application that displays translucent notecard windows via global hotkeys.

## Features

### System Tray Integration
- Runs in the system tray with a minimal footprint
- Right-click menu for configuration and exit
- No taskbar or dock presence
- Single instance enforcement

### Global Hotkeys
- Default: `Ctrl+Shift+[1-9]` (customizable)
- Supports Windows key modifier
- Works across all applications
- Instant notecard display

### Translucent Notecards
- Adjustable opacity (20-100%)
- Click-through when not focused
- Auto-hide timer support
- Escape key or click to dismiss
- Windows 10/11 blur effects

### Display Options
- Customizable position and size
- Multiple font families
- Adjustable font size (10-36pt)
- Dark background for readability
- Per-monitor DPI awareness

## Prerequisites

- Windows 7 or later (Windows 10/11 recommended)
- Visual Studio 2019+ Build Tools or Visual Studio
- Rust 1.70+ with MSVC toolchain

## Building

1. **Install Rust for Windows:**
   ```powershell
   # Download from https://rustup.rs/
   # Choose MSVC toolchain during installation
   ```

2. **Clone and build:**
   ```powershell
   cd windows
   cargo build --release
   ```

3. **The executable will be at:**
   ```
   target/release/notecognito.exe
   ```

## Installation

1. **Copy the executable** to a permanent location:
   ```powershell
   mkdir "C:\Program Files\Notecognito"
   copy target\release\notecognito.exe "C:\Program Files\Notecognito\"
   ```

2. **Add icon file** (required for system tray):
   ```powershell
   copy assets\icon.ico "C:\Program Files\Notecognito\"
   ```

3. **Run the application:**
    - Double-click `notecognito.exe`
    - Or add to Windows startup (configurable in app)

## Usage

### System Tray Menu
- **Configure**: Opens the Electron configuration UI
- **Quit**: Exits the application

### Hotkeys
- Press `Ctrl+Shift+[1-9]` to display notecards
- Only notecards with content will appear
- Hotkey modifiers can be customized in configuration

### Dismissing Notecards
- Click on the notecard
- Press Escape key
- Wait for auto-hide timer (if configured)

## Configuration

The app connects to the Notecognito core service for configuration. Ensure the core IPC server is running:

```powershell
# In the core directory
cargo run --bin notecognito-ipc-server
```

Configuration is stored in:
```
%APPDATA%\notecognito\config.json
```

## Project Structure

```
windows/
├── Cargo.toml           # Project configuration
├── build.rs             # Build script for resources
├── src/
│   ├── main.rs          # Application entry point
│   ├── hotkey.rs        # Global hotkey management
│   ├── ipc_client.rs    # Communication with core
│   ├── notecard_window.rs   # Window creation and rendering
│   └── platform_impl.rs     # Windows platform implementation
└── assets/
    └── icon.ico         # Application icon
```

## Technical Details

### Window Management
- Uses Win32 API for window creation
- `WS_EX_LAYERED` for transparency
- `WS_EX_TOPMOST` to stay on top
- `WS_EX_TOOLWINDOW` to hide from taskbar
- `WS_EX_NOACTIVATE` to prevent focus stealing

### Hotkey Registration
- Uses `RegisterHotKey` Win32 API
- Message loop in separate thread
- Unique IDs for each notecard

### Rendering
- GDI for text rendering
- ClearType font smoothing
- DWM blur effects on Windows 10/11
- Per-monitor DPI scaling

## Troubleshooting

### "Failed to register hotkey"
- Another application may be using the same hotkey
- Try changing the modifier keys in configuration
- Some keys may be reserved by Windows

### "Failed to connect to core service"
- Ensure the core IPC server is running
- Check Windows Firewall isn't blocking localhost:7855
- The app will run in standalone mode if core is unavailable

### Notecard doesn't appear
- Check the notecard has content
- Verify hotkey registration succeeded
- Try running as administrator

### Blurry text
- Enable ClearType in Windows settings
- Check display scaling settings
- Try different font families

## Security Considerations

- Hotkeys are registered globally but can't capture passwords
- No network connections except localhost IPC
- Configuration stored in user-specific directory
- Single instance enforcement via mutex

## Development

### Debug Build
```powershell
cargo build
set RUST_LOG=debug
target\debug\notecognito.exe
```

### Testing Hotkeys
The app logs all hotkey registrations and presses to help with debugging.

### Adding Features
- Implement new features in the `PlatformInterface` trait
- Update both the platform implementation and window manager
- Test with different Windows versions and DPI settings

## Distribution

1. **Build optimized binary:**
   ```powershell
   cargo build --release
   ```

2. **Create installer** (using NSIS or similar)

3. **Sign the executable** for SmartScreen compatibility

4. **Include required files:**
    - `notecognito.exe`
    - `icon.ico`
    - `notecognito-config.exe` (Electron app)
    - Visual C++ Redistributables