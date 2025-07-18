const { contextBridge, ipcRenderer } = require('electron');

// Expose protected methods that allow the renderer process to use
// the ipcRenderer without exposing the entire object
contextBridge.exposeInMainWorld('notecognitoAPI', {
  // Connect to the core service
  connectToCore: () => ipcRenderer.invoke('connect-to-core'),

  // Get current configuration
  getConfiguration: () => ipcRenderer.invoke('get-configuration'),

  // Update a single notecard
  updateNotecard: (notecard) => ipcRenderer.invoke('update-notecard', notecard),

  // Save the entire configuration
  saveConfiguration: (config) => ipcRenderer.invoke('save-configuration', config),

  // Listen for menu events
  onMenuAction: (callback) => {
    ipcRenderer.on('menu-save', () => callback('save'));
    ipcRenderer.on('menu-about', () => callback('about'));
  },

  // Platform detection
  platform: process.platform,

  // Get platform-specific hotkey display names
  getHotkeyDisplayName: (modifier) => {
    switch (modifier) {
      case 'Control':
        return process.platform === 'darwin' ? '⌃' : 'Ctrl';
      case 'Alt':
        return process.platform === 'darwin' ? '⌥' : 'Alt';
      case 'Shift':
        return process.platform === 'darwin' ? '⇧' : 'Shift';
      case 'Command':
        return process.platform === 'darwin' ? '⌘' : '';
      case 'Windows':
        return process.platform === 'win32' ? '⊞' : '';
      default:
        return modifier;
    }
  }
});