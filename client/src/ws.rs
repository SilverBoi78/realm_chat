use std::sync::{Arc, Mutex};

use common::{ChatMessage, WsMessage, models::{DirectMessage, FriendRequest}};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use uuid::Uuid;

pub struct WsClient {
    pub send_tx: mpsc::UnboundedSender<WsMessage>,
    pub recv_messages: Arc<Mutex<Vec<ChatMessage>>>,
    pub recv_dms: Arc<Mutex<Vec<DirectMessage>>>,
    pub recv_friend_requests: Arc<Mutex<Vec<FriendRequest>>>,
}

impl WsClient {
    pub fn spawn(token: String, rt: &tokio::runtime::Handle) -> Self {
        let (send_tx, mut send_rx) = mpsc::unbounded_channel::<WsMessage>();
        let recv_messages: Arc<Mutex<Vec<ChatMessage>>> = Arc::new(Mutex::new(Vec::new()));
        let recv_dms: Arc<Mutex<Vec<DirectMessage>>> = Arc::new(Mutex::new(Vec::new()));
        let recv_friend_requests: Arc<Mutex<Vec<FriendRequest>>> = Arc::new(Mutex::new(Vec::new()));

        let recv_clone = Arc::clone(&recv_messages);
        let recv_dms_clone = Arc::clone(&recv_dms);
        let recv_fr_clone = Arc::clone(&recv_friend_requests);

        rt.spawn(async move {
            let server_url = std::env::var("SERVER_URL").unwrap_or_else(|_| "http://91.98.84.90:8080".into());
            let ws_url = server_url
                .replace("http://", "ws://")
                .replace("https://", "wss://");
            let url = format!("{}/ws?token={}", ws_url, token);

            let (ws_stream, _) = match connect_async(&url).await {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("WebSocket connect failed: {e}");
                    return;
                }
            };

            let (mut ws_sink, mut ws_source) = ws_stream.split();

            let recv_task = tokio::spawn(async move {
                while let Some(Ok(msg)) = ws_source.next().await {
                    let text = match msg {
                        Message::Text(t) => t.to_string(),
                        Message::Close(_) => break,
                        _ => continue,
                    };
                    if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                        match ws_msg {
                            WsMessage::MessageReceived(chat_msg) => {
                                if let Ok(mut msgs) = recv_clone.lock() {
                                    msgs.push(chat_msg);
                                }
                            }
                            WsMessage::DirectMessageReceived(dm) => {
                                if let Ok(mut dms) = recv_dms_clone.lock() {
                                    dms.push(dm);
                                }
                            }
                            WsMessage::FriendRequestReceived(req) => {
                                if let Ok(mut frs) = recv_fr_clone.lock() {
                                    frs.push(req);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            });

            while let Some(msg) = send_rx.recv().await {
                if let Ok(json) = serde_json::to_string(&msg) {
                    if ws_sink.send(Message::Text(json.into())).await.is_err() {
                        break;
                    }
                }
            }

            recv_task.abort();
        });

        WsClient { send_tx, recv_messages, recv_dms, recv_friend_requests }
    }

    pub fn join(&self, location_id: Uuid) {
        let _ = self.send_tx.send(WsMessage::Join { location_id });
    }

    pub fn send_chat(&self, location_id: Uuid, content: String) {
        let _ = self.send_tx.send(WsMessage::Chat { location_id, content });
    }

    pub fn send_dm(&self, receiver_id: Uuid, content: String) {
        let _ = self.send_tx.send(WsMessage::SendDm { receiver_id, content });
    }

    pub fn drain_messages(&self) -> Vec<ChatMessage> {
        if let Ok(mut msgs) = self.recv_messages.lock() {
            std::mem::take(&mut *msgs)
        } else {
            Vec::new()
        }
    }

    pub fn drain_dms(&self) -> Vec<DirectMessage> {
        if let Ok(mut dms) = self.recv_dms.lock() {
            std::mem::take(&mut *dms)
        } else {
            Vec::new()
        }
    }

    pub fn drain_friend_requests(&self) -> Vec<FriendRequest> {
        if let Ok(mut frs) = self.recv_friend_requests.lock() {
            std::mem::take(&mut *frs)
        } else {
            Vec::new()
        }
    }
}
