use common::{
    models::{CharacterMode, DirectMessage, FriendRequest, Location, World},
    protocol::{
        AuthResponse, CreateLocationRequest, CreateWorldRequest, FriendsResponse,
        JoinWorldRequest, LoginRequest, RegisterRequest, SendFriendRequestBody,
    },
    ChatMessage,
};
use uuid::Uuid;

fn base_url() -> String {
    std::env::var("SERVER_URL").unwrap_or_else(|_| "http://91.98.84.90:8080".into())
}

pub async fn register(username: &str, password: &str) -> anyhow::Result<AuthResponse> {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/auth/register", base_url()))
        .json(&RegisterRequest { username: username.into(), password: password.into() })
        .send()
        .await?
        .error_for_status()?
        .json::<AuthResponse>()
        .await?;
    Ok(resp)
}

pub async fn login(username: &str, password: &str) -> anyhow::Result<AuthResponse> {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/auth/login", base_url()))
        .json(&LoginRequest { username: username.into(), password: password.into() })
        .send()
        .await?
        .error_for_status()?
        .json::<AuthResponse>()
        .await?;
    Ok(resp)
}

pub async fn list_worlds(token: &str) -> anyhow::Result<Vec<World>> {
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/worlds", base_url()))
        .bearer_auth(token)
        .send()
        .await?
        .error_for_status()?
        .json::<Vec<World>>()
        .await?;
    Ok(resp)
}

pub async fn create_world(token: &str, name: &str, description: &str, theme_id: &str) -> anyhow::Result<World> {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/worlds", base_url()))
        .bearer_auth(token)
        .json(&CreateWorldRequest {
            name: name.into(),
            description: description.into(),
            theme_id: theme_id.into(),
            character_mode: CharacterMode::Universal,
        })
        .send()
        .await?
        .error_for_status()?
        .json::<World>()
        .await?;
    Ok(resp)
}

pub async fn join_world(token: &str, world_id: Uuid) -> anyhow::Result<World> {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/worlds/{}/join", base_url(), world_id))
        .bearer_auth(token)
        .json(&JoinWorldRequest { invite_code: None })
        .send()
        .await?
        .error_for_status()?
        .json::<World>()
        .await?;
    Ok(resp)
}

pub async fn list_locations(token: &str, world_id: Uuid) -> anyhow::Result<Vec<Location>> {
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/worlds/{}/locations", base_url(), world_id))
        .bearer_auth(token)
        .send()
        .await?
        .error_for_status()?
        .json::<Vec<Location>>()
        .await?;
    Ok(resp)
}

pub async fn create_location(token: &str, world_id: Uuid, name: &str) -> anyhow::Result<Location> {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/worlds/{}/locations", base_url(), world_id))
        .bearer_auth(token)
        .json(&CreateLocationRequest { name: name.into() })
        .send()
        .await?
        .error_for_status()?
        .json::<Location>()
        .await?;
    Ok(resp)
}

pub async fn get_messages(token: &str, world_id: Uuid, loc_id: Uuid) -> anyhow::Result<Vec<ChatMessage>> {
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/worlds/{}/locations/{}/messages", base_url(), world_id, loc_id))
        .bearer_auth(token)
        .send()
        .await?
        .error_for_status()?
        .json::<Vec<ChatMessage>>()
        .await?;
    Ok(resp)
}

pub async fn send_friend_request(token: &str, username: &str) -> anyhow::Result<FriendRequest> {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/friends/request", base_url()))
        .bearer_auth(token)
        .json(&SendFriendRequestBody { username: username.into() })
        .send()
        .await?
        .error_for_status()?
        .json::<FriendRequest>()
        .await?;
    Ok(resp)
}

pub async fn accept_friend_request(token: &str, friendship_id: Uuid) -> anyhow::Result<FriendRequest> {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/friends/{}/accept", base_url(), friendship_id))
        .bearer_auth(token)
        .send()
        .await?
        .error_for_status()?
        .json::<FriendRequest>()
        .await?;
    Ok(resp)
}

pub async fn remove_friend(token: &str, friendship_id: Uuid) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    client
        .delete(format!("{}/friends/{}", base_url(), friendship_id))
        .bearer_auth(token)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

pub async fn list_friends(token: &str) -> anyhow::Result<FriendsResponse> {
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/friends", base_url()))
        .bearer_auth(token)
        .send()
        .await?
        .error_for_status()?
        .json::<FriendsResponse>()
        .await?;
    Ok(resp)
}

pub async fn get_dm_history(token: &str, peer_id: Uuid) -> anyhow::Result<Vec<DirectMessage>> {
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/friends/{}/messages", base_url(), peer_id))
        .bearer_auth(token)
        .send()
        .await?
        .error_for_status()?
        .json::<Vec<DirectMessage>>()
        .await?;
    Ok(resp)
}
