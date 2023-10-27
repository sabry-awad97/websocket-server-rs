use std::{
    collections::HashMap,
    process,
    sync::{Arc, RwLock},
};

use futures::StreamExt;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tokio::{
    net::{TcpListener, TcpStream},
    signal, task,
};
use tokio_tungstenite::{accept_async, tungstenite::Message};

#[derive(Debug, thiserror::Error)]
enum ServerError {
    #[error("IO error: {0}")]
    IO(#[from] tokio::io::Error),

    #[error("Tungstenite error: {0}")]
    Tungstenite(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("Serde error: {0}")]
    Serde(#[from] serde_json::Error),
}

#[derive(Debug, Deserialize, Serialize, Clone, Eq, Hash, PartialEq)]
#[serde(rename_all = "camelCase")]
enum EventType {
    Message,
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
    event_callbacks: Arc<RwLock<HashMap<EventType, Box<dyn Fn(Event<P>) + Send + Sync>>>>,
}

impl<P> WebSocketServer<P>
where
    P: DeserializeOwned + Clone + 'static,
{
    async fn new(address: &str) -> Self {
        let listener = TcpListener::bind(address).await.expect("Failed to bind");
        Self {
            listener: listener.into(),
            event_callbacks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn handle_client(&self, stream: TcpStream) -> Result<(), ServerError> {
        let ws_stream = accept_async(stream).await?;
        let (_, mut receiver) = ws_stream.split();

        loop {
            let msg = receiver.next().await.expect("Error reading message")?;

            match msg {
                Message::Text(text) => {
                    let event = serde_json::from_str::<Event<P>>(&text)?;
                    if let Some(callback) = self
                        .event_callbacks
                        .read()
                        .unwrap()
                        .get(&event.r#type.clone())
                    {
                        callback(event);
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

        Ok(())
    }

    fn on<F>(&self, event_type: EventType, callback: F)
    where
        F: Fn(Event<P>) + Send + Sync + 'static,
    {
        let mut callbacks = self.event_callbacks.write().unwrap(); // Acquiring write lock
        callbacks.insert(event_type, Box::new(callback));
    }

    async fn start(&self) -> Result<(), ServerError> {
        println!(
            "Server is listening on {}",
            self.listener.local_addr().unwrap()
        );

        loop {
            let (stream, _) = self.listener.accept().await?;

            let cloned_self = self.clone();

            task::spawn(async move {
                let _ = cloned_self.handle_client(stream).await;
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

    let _ = server.start().await;
}
