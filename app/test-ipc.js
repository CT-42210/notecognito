/**
 * Test script to verify IPC connection to Rust core
 * Run with: node test-ipc.js
 */

const net = require('net');

const IPC_PORT = 7855;
const IPC_HOST = '127.0.0.1';

class TestIpcClient {
  constructor() {
    this.socket = new net.Socket();
    this.connected = false;
  }

  connect() {
    return new Promise((resolve, reject) => {
      console.log(`Connecting to ${IPC_HOST}:${IPC_PORT}...`);

      this.socket.connect(IPC_PORT, IPC_HOST, () => {
        console.log('✓ Connected to IPC server');
        this.connected = true;
        resolve();
      });

      this.socket.on('data', (data) => {
        // Parse length-prefixed message
        let offset = 0;
        while (offset < data.length) {
          const messageLength = data.readUInt32LE(offset);
          const messageData = data.slice(offset + 4, offset + 4 + messageLength);
          const message = JSON.parse(messageData.toString());
          console.log('← Received:', JSON.stringify(message, null, 2));
          offset += 4 + messageLength;
        }
      });

      this.socket.on('error', (err) => {
        console.error('✗ Connection error:', err.message);
        reject(err);
      });

      this.socket.on('close', () => {
        console.log('Connection closed');
        this.connected = false;
      });

      setTimeout(() => {
        if (!this.connected) {
          reject(new Error('Connection timeout'));
        }
      }, 5000);
    });
  }

  sendMessage(message) {
    return new Promise((resolve, reject) => {
      if (!this.connected) {
        reject(new Error('Not connected'));
        return;
      }

      console.log('→ Sending:', JSON.stringify(message, null, 2));

      const jsonData = JSON.stringify(message);
      const buffer = Buffer.alloc(4 + jsonData.length);
      buffer.writeUInt32LE(jsonData.length, 0);
      buffer.write(jsonData, 4);

      this.socket.write(buffer, (err) => {
        if (err) {
          reject(err);
        } else {
          resolve();
        }
      });
    });
  }

  close() {
    this.socket.destroy();
  }
}

// Run tests
async function runTests() {
  const client = new TestIpcClient();

  try {
    // Test 1: Connect to server
    await client.connect();
    console.log('\n--- Test 1: Connection successful ---\n');

    // Test 2: Get configuration
    console.log('--- Test 2: Getting configuration ---');
    await client.sendMessage({
      id: Date.now().toString(),
      type: 'GetConfiguration'
    });

    // Wait for response
    await new Promise(resolve => setTimeout(resolve, 1000));

    // Test 3: Update a notecard
    console.log('\n--- Test 3: Updating notecard ---');
    await client.sendMessage({
      id: (Date.now() + 1).toString(),
      type: 'UpdateNotecard',
      notecard: {
        id: 1,
        content: 'Test notecard content from IPC test script!'
      }
    });

    // Wait for response
    await new Promise(resolve => setTimeout(resolve, 1000));

    // Test 4: Save configuration
    console.log('\n--- Test 4: Saving configuration ---');
    await client.sendMessage({
      id: (Date.now() + 2).toString(),
      type: 'GetConfiguration'
    });

    // Wait a bit more to see final response
    await new Promise(resolve => setTimeout(resolve, 2000));

    console.log('\n✓ All tests completed successfully!');

  } catch (error) {
    console.error('\n✗ Test failed:', error.message);
    console.error('\nMake sure the Rust IPC server is running:');
    console.error('  cd notecognito-core');
    console.error('  cargo run --bin notecognito-ipc-server');
  } finally {
    client.close();
    process.exit(0);
  }
}

// Run the tests
runTests();