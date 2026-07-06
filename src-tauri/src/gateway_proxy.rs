use futures_util::{SinkExt, StreamExt};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc::{self, UnboundedSender};
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Message;

struct GatewayConnection {
    sender: Option<UnboundedSender<Message>>,
    task: Option<JoinHandle<()>>,
}

pub struct GatewayProxy {
    connection: Mutex<GatewayConnection>,
}

impl Default for GatewayProxy {
    fn default() -> Self {
        Self {
            connection: Mutex::new(GatewayConnection {
                sender: None,
                task: None,
            }),
        }
    }
}

impl GatewayProxy {
    pub async fn connect(&self, app: AppHandle, url: String) -> Result<(), String> {
        self.disconnect()?;
        let (socket, _) = tokio::time::timeout(
            Duration::from_secs(15),
            tokio_tungstenite::connect_async(&url),
        )
        .await
        .map_err(|_| "Hermes did not open its control channel in time".to_string())?
        .map_err(|error| format!("Could not connect to Hermes: {error}"))?;
        let (mut writer, mut reader) = socket.split();
        let (sender, mut receiver) = mpsc::unbounded_channel::<Message>();
        let task = tokio::spawn(async move {
            let mut reason = "Hermes disconnected".to_string();
            loop {
                tokio::select! {
                    outgoing = receiver.recv() => {
                        let Some(message) = outgoing else {
                            break;
                        };
                        if writer.send(message).await.is_err() {
                            reason = "Papers could not send to Hermes".to_string();
                            break;
                        }
                    }
                    incoming = reader.next() => {
                        match incoming {
                            Some(Ok(Message::Text(text))) => {
                                let _ = app.emit("papers://gateway-frame", text.to_string());
                            }
                            Some(Ok(Message::Binary(bytes))) => {
                                if let Ok(text) = String::from_utf8(bytes.to_vec()) {
                                    let _ = app.emit("papers://gateway-frame", text);
                                }
                            }
                            Some(Ok(Message::Ping(payload))) => {
                                if writer.send(Message::Pong(payload)).await.is_err() {
                                    reason = "Hermes stopped responding".to_string();
                                    break;
                                }
                            }
                            Some(Ok(Message::Close(frame))) => {
                                reason = frame
                                    .map(|frame| frame.reason.to_string())
                                    .filter(|reason| !reason.is_empty())
                                    .unwrap_or_else(|| "Hermes closed its control channel".to_string());
                                break;
                            }
                            Some(Ok(_)) => {}
                            Some(Err(error)) => {
                                reason = format!("Hermes control channel failed: {error}");
                                break;
                            }
                            None => break,
                        }
                    }
                }
            }
            let _ = app.emit("papers://gateway-closed", reason);
        });

        let mut connection = self
            .connection
            .lock()
            .map_err(|_| "Hermes connection lock failed".to_string())?;
        connection.sender = Some(sender);
        connection.task = Some(task);
        Ok(())
    }

    pub fn send(&self, frame: String) -> Result<(), String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "Hermes connection lock failed".to_string())?;
        let sender = connection
            .sender
            .as_ref()
            .ok_or_else(|| "Hermes is not connected".to_string())?;
        sender
            .send(Message::Text(frame.into()))
            .map_err(|_| "Hermes is not connected".to_string())
    }

    pub fn disconnect(&self) -> Result<(), String> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| "Hermes connection lock failed".to_string())?;
        connection.sender.take();
        if let Some(task) = connection.task.take() {
            task.abort();
        }
        Ok(())
    }
}
