use common::models::{ChatMessage, DirectMessage, FriendRequest, Location, World};
use common::protocol::FriendEntry;
use uuid::Uuid;

#[derive(Default, PartialEq)]
pub enum Screen {
    #[default]
    Login,
    Register,
    Main,
}

#[derive(Default, PartialEq)]
pub enum MainView {
    #[default]
    Welcome,
    LocationChat,
    DirectMessage,
}

#[derive(Default)]
pub struct AuthState {
    pub token: String,
    pub user_id: Option<Uuid>,
    pub username: String,
    pub login_username: String,
    pub login_password: String,
    pub register_username: String,
    pub register_password: String,
    pub error: Option<String>,
}

#[derive(Default)]
pub struct ChatState {
    pub worlds: Vec<World>,
    pub selected_world: Option<Uuid>,
    pub locations: Vec<Location>,
    pub selected_location: Option<Uuid>,
    pub messages: Vec<ChatMessage>,
    pub compose_text: String,
    pub new_world_name: String,
    pub new_world_description: String,
    pub new_world_theme: String,
    pub new_location_name: String,
    pub show_create_world: bool,
    pub show_create_location: bool,
}

#[derive(Default)]
pub struct FriendsState {
    pub friends: Vec<FriendEntry>,
    pub pending_incoming: Vec<FriendRequest>,
    pub pending_outgoing: Vec<FriendRequest>,
    pub add_friend_input: String,
    pub add_friend_error: Option<String>,
    pub active_dm_peer: Option<Uuid>,
    pub dm_messages: Vec<DirectMessage>,
    pub dm_compose: String,
    pub unread_requests: usize,
}

impl FriendsState {
    pub fn peer_username<'a>(&'a self, peer_id: Uuid) -> &'a str {
        self.friends.iter()
            .find(|f| f.user_id == peer_id)
            .map(|f| f.username.as_str())
            .unwrap_or("Unknown")
    }
}

