use std::collections::HashMap;
use std::sync::Arc;
use std::{process, sync::Mutex};

use futures::StreamExt;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tokio::{
    net::{TcpListener, TcpStream},
    signal, task,
};
use tokio_tungstenite::{accept_async, tungstenite::Message};

#[derive(Debug, Deserialize, Serialize, Clone, Eq, Hash, PartialEq)]
enum EventType {
    Join,
    Message,
}

impl From<&str> for EventType {
    fn from(s: &str) -> Self {
        match s {
            "join" => EventType::Join,
            "message" => EventType::Message,
            _ => panic!("Unknown event type: {}", s),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Event<T> {
    #[serde(rename = "type")]
    r#type: EventType,
    payload: T,
}

#[derive(Clone)]
struct WebSocketServer<P> {
    listener: Arc<TcpListener>,

    #[allow(clippy::type_complexity)]
    event_callbacks: Arc<Mutex<HashMap<EventType, Box<dyn Fn(Event<P>) + Send>>>>,
}

impl<P> WebSocketServer<P>
where
    P: DeserializeOwned + Clone + 'static,
{
    async fn new(address: &str) -> Self {
        let listener = TcpListener::bind(address).await.expect("Failed to bind");
        Self {
            listener: listener.into(),
            event_callbacks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn handle_client(&self, stream: TcpStream) {
        let ws_stream = accept_async(stream).await.expect("Error during handshake");
        let (_, mut receiver) = ws_stream.split();

        let callbacks = Arc::clone(&self.event_callbacks);
        loop {
            let msg = receiver
                .next()
                .await
                .expect("Error reading message")
                .expect("No message received");

            match msg {
                Message::Text(text) => {
                    if let Ok(event) = serde_json::from_str::<Event<P>>(&text) {
                        if let Some(callback) = callbacks.lock().unwrap().get(&event.r#type.clone())
                        {
                            callback(event);
                        }
                    }
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

    fn on<F>(&self, event_type: EventType, callback: F)
    where
        F: Fn(Event<P>) + Send + 'static,
    {
        let mut callbacks = self.event_callbacks.lock().unwrap();
        callbacks.insert(event_type, Box::new(callback));
    }

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

    server.on(EventType::Message, |event: Event<String>| {
        println!("Received text message: {}", event.payload.trim());
    });

    task::spawn(async move {
        signal::ctrl_c().await.expect("Failed to bind SIGINT");
        println!("Received SIGINT, shutting down gracefully...");
        process::exit(0);
    });

    server.start().await;
}
