use std::sync::Arc;

use axum::{
    Router,
    extract::{
        Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::{IntoResponse, Response},
    routing::get,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use uuid::Uuid;

use common::{ChatMessage, WsMessage, models::DirectMessage};
use crate::{auth::decode_jwt, db, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new().route("/ws", get(ws_handler))
}

#[derive(Deserialize)]
struct WsQuery {
    token: String,
}

async fn ws_handler(
    State(state): State<AppState>,
    Query(query): Query<WsQuery>,
    ws: WebSocketUpgrade,
) -> Response {
    let claims = match decode_jwt(&query.token, &state.jwt_secret) {
        Ok(c) => c,
        Err(_) => return (axum::http::StatusCode::UNAUTHORIZED, "Invalid token").into_response(),
    };
    let user_id: Uuid = match claims.sub.parse() {
        Ok(id) => id,
        Err(_) => return (axum::http::StatusCode::UNAUTHORIZED, "Invalid token").into_response(),
    };
    ws.on_upgrade(move |socket| handle_socket(socket, state, user_id, claims.username))
}

async fn handle_socket(socket: WebSocket, state: AppState, user_id: Uuid, username: String) {
    let (mut ws_send, mut ws_recv) = socket.split();
    let (out_tx, mut out_rx) = tokio::sync::mpsc::unbounded_channel::<String>();

    let send_task = tokio::spawn(async move {
        while let Some(json) = out_rx.recv().await {
            if ws_send.send(Message::Text(json.into())).await.is_err() {
                break;
            }
        }
    });

    {
        let hub = Arc::clone(&state.hub);
        let tx = out_tx.clone();
        tokio::spawn(async move {
            let mut rx = hub.subscribe_user(user_id);
            while let Ok(msg) = rx.recv().await {
                if let Ok(json) = serde_json::to_string(&msg) {
                    if tx.send(json).is_err() {
                        break;
                    }
                }
            }
        });
    }

    while let Some(Ok(raw)) = ws_recv.next().await {
        let text = match raw {
            Message::Text(t) => t.to_string(),
            Message::Close(_) => break,
            _ => continue,
        };

        let ws_msg: WsMessage = match serde_json::from_str(&text) {
            Ok(m) => m,
            Err(_) => continue,
        };

        match ws_msg {
            WsMessage::Join { location_id } => {
                let hub = Arc::clone(&state.hub);
                let tx = out_tx.clone();
                tokio::spawn(async move {
                    let mut rx = hub.subscribe(location_id);
                    while let Ok(msg) = rx.recv().await {
                        let envelope = WsMessage::MessageReceived(msg);
                        if let Ok(json) = serde_json::to_string(&envelope) {
                            if tx.send(json).is_err() {
                                break;
                            }
                        }
                    }
                });
            }
            WsMessage::Chat { location_id, content } => {
                let world_id = match db::get_location_world(&state.pool, location_id).await {
                    Ok(Some(id)) => id,
                    _ => continue,
                };

                let msg_id = Uuid::new_v4();
                if let Err(e) = db::insert_message(
                    &state.pool, msg_id, world_id, location_id, user_id, &username, &content,
                ).await {
                    tracing::error!("insert_message failed: {e}");
                    continue;
                }

                let chat_msg = ChatMessage {
                    id: msg_id,
                    world_id,
                    location_id,
                    sender_id: user_id,
                    sender_name: username.clone(),
                    content,
                    timestamp: chrono::Utc::now(),
                };
                state.hub.publish(location_id, chat_msg);
            }
            WsMessage::SendDm { receiver_id, content } => {
                if !db::are_friends(&state.pool, user_id, receiver_id).await.unwrap_or(false) {
                    continue;
                }
                let msg_id = Uuid::new_v4();
                if db::insert_dm(&state.pool, msg_id, user_id, receiver_id, &content).await.is_err() {
                    continue;
                }
                let dm = DirectMessage {
                    id: msg_id,
                    sender_id: user_id,
                    sender_name: username.clone(),
                    receiver_id,
                    content,
                    timestamp: chrono::Utc::now(),
                };
                state.hub.notify_user(receiver_id, WsMessage::DirectMessageReceived(dm.clone()));
                state.hub.notify_user(user_id, WsMessage::DirectMessageReceived(dm));
            }
            WsMessage::Ping => {
                if let Ok(json) = serde_json::to_string(&WsMessage::Pong) {
                    let _ = out_tx.send(json);
                }
            }
            _ => {}
        }
    }

    send_task.abort();
}
