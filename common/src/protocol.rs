use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::{ChatMessage, DirectMessage, FriendRequest};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum WsMessage {
    Join { location_id: Uuid },
    Leave { location_id: Uuid },
    Chat { location_id: Uuid, content: String },
    MessageReceived(ChatMessage),
    SendDm { receiver_id: Uuid, content: String },
    DirectMessageReceived(DirectMessage),
    FriendRequestReceived(FriendRequest),
    Error { message: String },
    Ping,
    Pong,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub user_id: Uuid,
    pub username: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateWorldRequest {
    pub name: String,
    pub description: String,
    pub theme_id: String,
    pub character_mode: crate::models::CharacterMode,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateLocationRequest {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JoinWorldRequest {
    pub invite_code: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiError {
    pub error: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendFriendRequestBody {
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendEntry {
    pub user_id: Uuid,
    pub username: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FriendsResponse {
    pub friends: Vec<FriendEntry>,
    pub pending_incoming: Vec<crate::models::FriendRequest>,
    pub pending_outgoing: Vec<crate::models::FriendRequest>,
}
