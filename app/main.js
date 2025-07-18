const { app, BrowserWindow, ipcMain, Menu, shell } = require('electron');
const path = require('path');
const net = require('net');

// IPC Configuration
const IPC_PORT = 7855;
const IPC_HOST = '127.0.0.1';

let mainWindow;
let ipcClient;

// Create the main window
function createWindow() {
  mainWindow = new BrowserWindow({
    width: 900,
    height: 700,
    minWidth: 800,
    minHeight: 600,
    webPreferences: {
      nodeIntegration: false,
      contextIsolation: true,
      preload: path.join(__dirname, 'preload.js')
    },
    icon: path.join(__dirname, 'assets', 'icon.png'),
    titleBarStyle: process.platform === 'darwin' ? 'hiddenInset' : 'default',
    backgroundColor: '#f8f9fa'
  });

  mainWindow.loadFile('index.html');

  // Open links in external browser
  mainWindow.webContents.setWindowOpenHandler(({ url }) => {
    shell.openExternal(url);
    return { action: 'deny' };
  });

  // Set up application menu
  const template = [
    {
      label: 'File',
      submenu: [
        {
          label: 'Save Configuration',
          accelerator: 'CmdOrCtrl+S',
          click: () => mainWindow.webContents.send('menu-save')
        },
        { type: 'separator' },
        {
          label: 'Quit',
          accelerator: process.platform === 'darwin' ? 'Cmd+Q' : 'Ctrl+Q',
          click: () => {
            quitApp();
          }
        }
      ]
    },
    {
      label: 'Edit',
      submenu: [
        { role: 'undo' },
        { role: 'redo' },
        { type: 'separator' },
        { role: 'cut' },
        { role: 'copy' },
        { role: 'paste' }
      ]
    },
    {
      label: 'View',
      submenu: [
        { role: 'reload' },
        { role: 'toggleDevTools' },
        { type: 'separator' },
        { role: 'resetZoom' },
        { role: 'zoomIn' },
        { role: 'zoomOut' }
      ]
    },
    {
      label: 'Help',
      submenu: [
        {
          label: 'About Notecognito',
          click: () => mainWindow.webContents.send('menu-about')
        },
        {
          label: 'Documentation',
          click: () => shell.openExternal('https://github.com/notecognito/docs')
        }
      ]
    }
  ];

  if (process.platform === 'darwin') {
    template.unshift({
      label: app.getName(),
      submenu: [
        { role: 'about' },
        { type: 'separator' },
        { role: 'services', submenu: [] },
        { type: 'separator' },
        { role: 'hide' },
        { role: 'hideOthers' },
        { role: 'unhide' },
        { type: 'separator' },
        {
          label: 'Quit ' + app.getName(),
          accelerator: 'Command+Q',
          click: () => {
            quitApp();
          }
        }
      ]
    });
  }

  const menu = Menu.buildFromTemplate(template);
  Menu.setApplicationMenu(menu);

  // Set up custom dock menu on macOS
  if (process.platform === 'darwin') {
    const dockMenu = Menu.buildFromTemplate([
      {
        label: 'Quit ' + app.getName(),
        click: () => {
          quitApp();
        }
      }
    ]);
    app.dock.setMenu(dockMenu);
  }

  // Handle window close event
  mainWindow.on('close', (event) => {
    if (process.platform === 'darwin') {
      // On macOS, hide the window instead of closing unless we're quitting the app
      if (!app.isQuitting) {
        event.preventDefault();
        mainWindow.hide();
        return false;
      }
    }
    // Allow the window to close
    return true;
  });

  // Cleanup on window close
  mainWindow.on('closed', () => {
    mainWindow = null;
    if (ipcClient) {
      ipcClient.destroy();
      ipcClient = null;
    }
  });
}

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    quitApp();
  }
});

// IPC Client for communicating with Rust core
class IpcClient {
  constructor() {
    this.socket = new net.Socket();
    this.connected = false;
    this.messageHandlers = new Map();
    this.buffer = Buffer.alloc(0);
  }

  connect() {
    return new Promise((resolve, reject) => {
      this.socket.connect(IPC_PORT, IPC_HOST, () => {
        console.log('Connected to IPC server');
        this.connected = true;
        resolve();
      });

      this.socket.on('data', (data) => {
        this.buffer = Buffer.concat([this.buffer, data]);
        this.processBuffer();
      });

      this.socket.on('error', (err) => {
        console.error('IPC connection error:', err);
        this.connected = false;
        reject(err);
      });

      this.socket.on('close', () => {
        console.log('IPC connection closed');
        this.connected = false;
      });

      // Set timeout for connection
      setTimeout(() => {
        if (!this.connected) {
          reject(new Error('Connection timeout'));
        }
      }, 5000);
    });
  }

  processBuffer() {
    while (this.buffer.length >= 4) {
      const messageLength = this.buffer.readUInt32LE(0);

      if (this.buffer.length >= 4 + messageLength) {
        const messageData = this.buffer.slice(4, 4 + messageLength);
        this.buffer = this.buffer.slice(4 + messageLength);

        try {
          const message = JSON.parse(messageData.toString());
          this.handleMessage(message);
        } catch (err) {
          console.error('Failed to parse message:', err);
        }
      } else {
        break;
      }
    }
  }

  handleMessage(message) {
    const handler = this.messageHandlers.get(message.id);
    if (handler) {
      handler(message);
      this.messageHandlers.delete(message.id);
    }
  }

  sendMessage(messageType, data = {}) {
    return new Promise((resolve, reject) => {
      if (!this.connected) {
        reject(new Error('Not connected to IPC server'));
        return;
      }

      const message = {
        id: Date.now().toString(),
        type: messageType,
        ...data
      };

      this.messageHandlers.set(message.id, (response) => {
        if (response.type === 'Error') {
          reject(new Error(response.message));
        } else {
          resolve(response);
        }
      });

      const jsonData = JSON.stringify(message);
      const buffer = Buffer.alloc(4 + jsonData.length);
      buffer.writeUInt32LE(jsonData.length, 0);
      buffer.write(jsonData, 4);

      this.socket.write(buffer);
    });
  }

  destroy() {
    if (this.socket) {
      this.socket.destroy();
    }
  }
}

// IPC Handlers for renderer process
ipcMain.handle('connect-to-core', async () => {
  try {
    if (!ipcClient) {
      ipcClient = new IpcClient();
      await ipcClient.connect();
    }
    return { success: true };
  } catch (err) {
    console.error('Failed to connect to core:', err);
    return { success: false, error: err.message };
  }
});

ipcMain.handle('get-configuration', async () => {
  try {
    if (!ipcClient || !ipcClient.connected) {
      throw new Error('Not connected to core service');
    }

    const response = await ipcClient.sendMessage('GetConfiguration');
    return { success: true, config: response.config };
  } catch (err) {
    console.error('Failed to get configuration:', err);
    return { success: false, error: err.message };
  }
});

ipcMain.handle('update-notecard', async (event, notecard) => {
  try {
    if (!ipcClient || !ipcClient.connected) {
      throw new Error('Not connected to core service');
    }

    await ipcClient.sendMessage('UpdateNotecard', { notecard });
    return { success: true };
  } catch (err) {
    console.error('Failed to update notecard:', err);
    return { success: false, error: err.message };
  }
});

ipcMain.handle('save-configuration', async (event, config) => {
  try {
    if (!ipcClient || !ipcClient.connected) {
      throw new Error('Not connected to core service');
    }

    await ipcClient.sendMessage('SaveConfiguration', { config });
    return { success: true };
  } catch (err) {
    console.error('Failed to save configuration:', err);
    return { success: false, error: err.message };
  }
});

// App event handlers
app.whenReady().then(createWindow);

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    quitApp();
  }
});

app.on('activate', () => {
  if (mainWindow === null) {
    createWindow();
  } else if (process.platform === 'darwin') {
    // On macOS, show the window when clicking the dock icon
    mainWindow.show();
  }
});

// Override all quit events to use our custom handler
app.on('before-quit', (event) => {
  console.log('before-quit event triggered');
  if (!app.isQuitting) {
    event.preventDefault();
    quitApp();
  }
});

app.on('will-quit', (event) => {
  console.log('will-quit event triggered');
  if (!app.isQuitting) {
    event.preventDefault();
    quitApp();
  }
});

// Proper quit function that ensures clean shutdown
function quitApp() {
  console.log('Quitting application...');
  app.isQuitting = true;

  // Close IPC connection first
  if (ipcClient) {
    console.log('Closing IPC connection...');
    ipcClient.destroy();
    ipcClient = null;
  }

  // Close window if it exists
  if (mainWindow) {
    console.log('Closing main window...');
    mainWindow.destroy();
    mainWindow = null;
  }

  // Force quit the app
  console.log('Forcing app quit...');
  app.quit();

  // If app.quit() doesn't work, force exit
  setTimeout(() => {
    console.log('Force exiting process...');
    process.exit(0);
  }, 1000);
}
