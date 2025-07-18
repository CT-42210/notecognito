# Notecognito Electron App Structure

## Directory Layout

```
notecognito-electron/
├── package.json              # NPM package configuration
├── main.js                   # Main process (backend)
├── preload.js               # Preload script (bridge)
├── renderer.js              # Renderer process (frontend logic)
├── index.html               # Main UI
├── assets/                  # Application assets
│   ├── icon.png            # App icon (512x512)
│   ├── icon.icns           # macOS icon
│   └── icon.ico            # Windows icon
└── node_modules/           # Dependencies (after npm install)
    ├── bootstrap/
    ├── bootstrap-icons/
    └── electron/
```

## Setup Instructions

1. **Create the project directory:**
   ```bash
   mkdir notecognito-electron
   cd notecognito-electron
   ```

2. **Copy all the created files into the directory**

3. **Install dependencies:**
   ```bash
   npm install
   ```

4. **Create the assets directory:**
   ```bash
   mkdir assets
   ```

5. **Add icon files** (you'll need to create these):
   - `icon.png` - 512x512 PNG for Linux
   - `icon.icns` - macOS icon (use Icon Composer or iconutil)
   - `icon.ico` - Windows icon (use an online converter)

## Running the Application

1. **Ensure the Rust IPC server is running:**
   ```bash
   # In the core directory
   cargo run --bin notecognito-ipc-server
   ```

2. **Start the Electron app:**
   ```bash
   npm start
   ```

## Building for Distribution

1. **Build for current platform:**
   ```bash
   npm run build
   ```

2. **Build for all platforms:**
   ```bash
   npm run dist
   ```

The built applications will be in the `dist` directory.

## Features Implemented

### Core Features
- ✅ Connection to Rust core via TCP IPC
- ✅ 9 notecards (keyboard shortcuts 1-9)
- ✅ Rich text editor for notecard content
- ✅ Character count with 10,000 limit
- ✅ Save/load configuration

### Display Settings
- ✅ Opacity control (20-100%)
- ✅ Font size adjustment (10-36pt)
- ✅ Font family selection
- ✅ Auto-hide timer (0-30 seconds)
- ✅ Algorithmic spacing option

### Global Settings
- ✅ Launch on startup toggle
- ✅ Customizable hotkey modifiers
- ✅ Platform-specific hotkey display

### UI/UX
- ✅ Bootstrap 5 styling
- ✅ Connection status indicator
- ✅ Unsaved changes warning
- ✅ Toast notifications
- ✅ Loading overlay
- ✅ Responsive design
- ✅ Platform-specific adjustments (macOS titlebar)

## Next Steps

1. **Test the connection** between Electron and the Rust core
2. **Implement the platform-specific apps** (Windows tray, macOS menu bar)
3. **Add icon creation** for the app
4. **Set up code signing** for distribution
5. **Create installers** for each platform

## Security Considerations

- IPC communication is restricted to localhost only
- Context isolation is enabled in Electron
- No direct Node.js access from renderer
- Input validation for all user data
- Secure configuration file handling