use std::process;
use std::sync::Arc;

use futures::StreamExt;
use tokio::{
    net::{TcpListener, TcpStream},
    signal, task,
};
use tokio_tungstenite::{accept_async, tungstenite::Message};

// Define a struct to represent the WebSocket server
#[derive(Debug, Clone)]
struct WebSocketServer {
    listener: Arc<TcpListener>,
}

impl WebSocketServer {
    async fn new(address: &str) -> Self {
        let listener = TcpListener::bind(address).await.expect("Failed to bind");

        Self {
            listener: listener.into(),
        }
    }

    // Method to handle a client's connection
    async fn handle_client(&self, stream: TcpStream) {
        let mut ws_stream = accept_async(stream).await.expect("Error during handshake");

        loop {
            let msg = ws_stream
                .next()
                .await
                .expect("Error reading message")
                .expect("No message received");

            match msg {
                Message::Text(text) => {
                    println!("Received text message: {}", text.trim());
                }
                Message::Binary(bin) => {
                    println!("Received Binary message: {:?}", bin);
                }
                Message::Close(_) => {
                    break;
                }
                _ => {}
            }
        }
    }

    // Method to start the server and handle incoming connections
    async fn start(&self) {
        println!(
            "Server is listening on {}",
            self.listener.local_addr().unwrap()
        );

        loop {
            let (stream, _) = self
                .listener
                .accept()
                .await
                .expect("Error accepting connection");

            let cloned_self = self.clone();

            task::spawn(async move {
                cloned_self.handle_client(stream).await;
            });
        }
    }
}

#[tokio::main]
async fn main() {
    let server = WebSocketServer::new("127.0.0.1:8080").await;

    task::spawn(async move {
        signal::ctrl_c().await.expect("Failed to bind SIGINT");
        println!("Received SIGINT, shutting down gracefully...");
        process::exit(0);
    });

    server.start().await;
}
