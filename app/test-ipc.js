const net = require('net');

const HOST = '127.0.0.1';
const PORT = 7855;

const client = new net.Socket();

let receiveBuffer = Buffer.alloc(0);

function sendMessage(type, payload = {}) {
  const message = {
    id: Date.now().toString(),
    type: type,
    ...payload,
  };
  const jsonMessage = JSON.stringify(message);
  const messageBuffer = Buffer.from(jsonMessage, 'utf8');

  // Create a 4-byte length prefix (little-endian)
  const lengthBuffer = Buffer.alloc(4);
  lengthBuffer.writeUInt32LE(messageBuffer.length, 0);

  // Send length prefix + message
  client.write(Buffer.concat([lengthBuffer, messageBuffer]));
  console.log('→ Sending:', jsonMessage);
}

function processBuffer() {
  while (true) {
    // Need at least 4 bytes for the length prefix
    if (receiveBuffer.length < 4) {
      return;
    }

    const messageLength = receiveBuffer.readUInt32LE(0);
    const totalLength = 4 + messageLength;

    // Check if the full message has been received
    if (receiveBuffer.length < totalLength) {
      return;
    }

    // Extract and parse the JSON message
    const messageJson = receiveBuffer.subarray(4, totalLength).toString('utf8');
    try {
      const message = JSON.parse(messageJson);
      console.log('← Received:', message);
    } catch (e) {
      console.error('Error parsing JSON:', e);
      console.error('Invalid JSON string:', messageJson);
    }


    // Remove the processed message from the buffer
    receiveBuffer = receiveBuffer.subarray(totalLength);
  }
}

client.connect(PORT, HOST, () => {
  console.log(`Connected to IPC server at ${HOST}:${PORT}`);
  console.log('--- Test 1: Connection successful ---');

  // Test getting configuration
  console.log('\n--- Test 2: Getting configuration ---');
  sendMessage('GetConfiguration');
});

client.on('data', (chunk) => {
  // Append new data to the buffer and try to process it
  receiveBuffer = Buffer.concat([receiveBuffer, chunk]);
  processBuffer();
});

client.on('close', () => {
  console.log('Connection closed');
});

client.on('error', (err) => {
  console.error('Socket error:', err);
});