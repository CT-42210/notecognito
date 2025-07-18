# Notecognito Configuration UI

A Bootstrap-based Electron application for configuring Notecognito notecards.

## Prerequisites

- Node.js 16+ and npm
- The Notecognito core service running (Rust IPC server)
- Platform-specific icon files in the `assets` directory

## Installation

1. Clone or download this directory
2. Install dependencies:
   ```bash
   npm install
   ```

3. Ensure the Rust core service is built and running:
   ```bash
   # In the notecognito-core directory
   cargo build --release
   cargo run --bin notecognito-ipc-server
   ```

## Development

### Running the App

```bash
# Start the Electron app
npm start

# Or run in development mode with DevTools
npm run dev
```

### Testing IPC Connection

Before running the Electron app, you can test the IPC connection:

```bash
node test-ipc.js
```

This will verify that the Rust core service is running and accepting connections.

## Features

### Notecard Management
- **9 Notecards**: Quick access via number buttons (1-9)
- **Rich Text Editor**: Multi-line content support with monospace font
- **Character Limit**: 10,000 characters per notecard with live counter
- **Auto-save**: Changes are preserved when switching between notecards

### Display Customization
- **Opacity**: Adjust transparency from 20% to 100% with live preview
- **Font Size**: Scale from 10pt to 36pt
- **Font Family**: Choose from system and common fonts
- **Auto-hide**: Set timer from manual dismiss (0) to 30 seconds
- **Algorithmic Spacing**: Toggle for improved readability

### Global Settings
- **Launch on Startup**: Configure app to start with system
- **Hotkey Modifiers**: Customize keyboard shortcuts (Ctrl, Alt, Shift)
- **Platform-aware**: Displays correct modifier keys for your OS

### User Interface
- **Modern Design**: Clean Bootstrap 5 interface
- **Connection Status**: Real-time indicator for core service
- **Save Confirmation**: Visual feedback and toast notifications
- **Unsaved Changes**: Warning before closing with pending changes
- **Responsive Layout**: Adapts to different window sizes

## Building for Distribution

### Package for Current Platform
```bash
npm run build
```

### Create Distributables
```bash
npm run dist
```

Built applications will be in the `dist` directory.

## Project Structure

```
├── main.js          # Main process - handles window and IPC
├── preload.js       # Bridge between main and renderer
├── renderer.js      # UI logic and user interactions
├── index.html       # User interface layout
├── test-ipc.js      # IPC connection test script
├── package.json     # Project configuration
└── assets/          # Application icons
    ├── icon.png     # Linux icon (512x512)
    ├── icon.icns    # macOS icon
    └── icon.ico     # Windows icon
```

## Troubleshooting

### "Failed to connect to Notecognito service"
- Ensure the Rust IPC server is running
- Check that port 7855 is not blocked
- Try running `test-ipc.js` to diagnose

### Blank Window
- Check the DevTools console (View → Toggle Developer Tools)
- Verify all files are in the correct locations
- Ensure Bootstrap CSS is loading correctly

### Hotkeys Not Working
- The configuration UI only sets up hotkeys
- The actual hotkey functionality requires the platform-specific app (Windows/macOS)

## Security

- IPC communication restricted to localhost
- Context isolation enabled
- No direct Node.js access from renderer
- Input validation on all user data

## Contributing

When modifying the app:
1. Test IPC communication with `test-ipc.js`
2. Verify all 9 notecards save correctly
3. Test on target platforms
4. Update version in `package.json`

## License

MIT License - See LICENSE file for details