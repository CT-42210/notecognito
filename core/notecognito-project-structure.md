# Notecognito Core Library - Project Structure

## Directory Layout

```
notecognito-core/
├── Cargo.toml                    # Rust project configuration
├── cbindgen.toml                 # C header generation config
├── build.sh                      # Build script
├── README.md                     # Core library documentation
├── notecognito.h                 # C header for FFI
├── src/
│   ├── lib.rs                    # Library entry point
│   ├── error.rs                  # Error handling
│   ├── notecard.rs               # Notecard data structures
│   ├── config.rs                 # Configuration management
│   ├── platform.rs               # Platform abstraction interface
│   ├── ipc.rs                    # IPC server and protocol
│   ├── ffi.rs                    # Foreign Function Interface
│   └── bin/
│       └── ipc_server.rs         # Standalone IPC server binary
└── examples/
    ├── test_client.rs            # Rust IPC test client
    ├── ipc_client.js             # Node.js IPC client example
    └── package.json              # Node.js example package
```

## Key Components

### 1. **Data Structures** (`notecard.rs`, `config.rs`)
- `Notecard`: ID (1-9) and content
- `Config`: Global settings including display properties, hotkeys, and startup behavior
- `DisplayProperties`: Opacity, position, size, font, auto-hide duration

### 2. **IPC Server** (`ipc.rs`)
- TCP server on localhost:7855
- JSON-based protocol with length-prefixed messages
- Message types: GetConfiguration, UpdateNotecard, SaveConfiguration
- Async implementation using Tokio

### 3. **Platform Interface** (`platform.rs`)
- Trait defining platform-specific operations
- Hotkey registration/unregistration
- Notecard display/hide
- Startup configuration
- Permission management

### 4. **Configuration Management** (`config.rs`)
- JSON file storage in platform-specific directories
- Automatic validation and serialization
- Thread-safe access through Arc<Mutex<>>

### 5. **FFI Support** (`ffi.rs`, `notecognito.h`)
- C-compatible interface for use from other languages
- Memory-safe string handling
- Error propagation

## Building and Running

1. **Build the core library**:
   ```bash
   cargo build --release
   ```

2. **Run the IPC server**:
   ```bash
   cargo run --bin notecognito-ipc-server
   ```

3. **Test with Rust client**:
   ```bash
   cargo run --example test_client
   ```

4. **Test with Node.js client**:
   ```bash
   cd examples
   npm run test-ipc
   ```

## Security Features

- IPC server binds only to localhost
- Message size limits (1MB max)
- Input validation on all operations
- No unsafe Rust code in core functionality
- Secure configuration file handling

## Next Steps

The core library is now ready. You can proceed to:
1. Build the Electron configuration UI
2. Implement the Windows system tray application
3. Implement the macOS menu bar application

Each platform implementation will:
- Use the IPC client to communicate with the core
- Implement the PlatformInterface trait for native functionality
- Handle platform-specific UI and hotkey management