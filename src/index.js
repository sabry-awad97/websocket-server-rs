const socket = new WebSocket('ws://127.0.0.1:8080');

socket.addEventListener('open', event => {
  console.log('Connected to server');
});

socket.addEventListener('message', event => {
  console.log('Received message from server:', event.data);
});

socket.addEventListener('close', event => {
  console.log('Connection closed');
});

// To send a message
function sendMessage(message) {
  socket.send(JSON.stringify({ type: 'message', payload: message }));
}

// Usage:
sendMessage('Hello, server!');

// To send a binary message
function sendBinaryMessage() {
  const buffer = new ArrayBuffer(4);
  const view = new DataView(buffer);
  view.setInt32(0, 123456, true);

  // Send the ArrayBuffer as a binary message
  socket.send(buffer);
}

// Usage:
sendBinaryMessage();
