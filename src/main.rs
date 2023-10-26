use std::process;

use futures::{SinkExt, StreamExt};
use tokio::{
    net::{TcpListener, TcpStream},
    signal, task,
};
use tokio_tungstenite::tungstenite::Message;

async fn handle_client(stream: TcpStream) {
    let mut ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during handshake");

    loop {
        let msg = ws_stream
            .next()
            .await
            .expect("Error reading message")
            .expect("No message received");

        match msg {
            Message::Text(text) => {
                println!("Received text message: {}", text);
                ws_stream
                    .send(Message::Text(text))
                    .await
                    .expect("Error sending message");
            }
            Message::Close(_) => {
                break;
            }
            _ => {}
        }
    }
}

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080")
        .await
        .expect("Failed to bind");

    println!("Server is listening on 127.0.0.1:8080");

    task::spawn(async move {
        signal::ctrl_c().await.expect("Failed to bind SIGINT");
        println!("Received SIGINT, shutting down gracefully...");
        process::exit(0);
    });

    while let Ok((stream, _)) = listener.accept().await {
        task::spawn(handle_client(stream));
    }
}
