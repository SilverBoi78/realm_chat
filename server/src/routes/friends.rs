use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{delete, get, post},
};
use uuid::Uuid;

use common::{
    models::{DirectMessage, FriendRequest, FriendStatus},
    protocol::{FriendEntry, FriendsResponse, SendFriendRequestBody},
};
use crate::{
    auth::{decode_jwt, extract_bearer},
    db,
    error::{AppError, Result},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/friends/request", post(send_request))
        .route("/friends/{id}/accept", post(accept_request))
        .route("/friends/{id}", delete(remove_friend))
        .route("/friends", get(list_friends))
        .route("/friends/{peer_id}/messages", get(get_dm_history))
}

fn require_auth(headers: &HeaderMap, secret: &str) -> Result<(Uuid, String)> {
    let token = extract_bearer(headers)
        .ok_or_else(|| AppError(StatusCode::UNAUTHORIZED, "Missing token".into()))?;
    let claims = decode_jwt(token, secret)?;
    let id = claims.sub.parse::<Uuid>()
        .map_err(|e| AppError(StatusCode::UNAUTHORIZED, e.to_string()))?;
    Ok((id, claims.username))
}

fn friendship_to_model(r: db::FriendshipRow) -> Result<FriendRequest> {
    let status = if r.status == "accepted" { FriendStatus::Accepted } else { FriendStatus::Pending };
    Ok(FriendRequest {
        id: r.id.parse().map_err(|e: uuid::Error| AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
        requester_id: r.requester_id.parse().map_err(|e: uuid::Error| AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
        requester_name: r.requester_name,
        addressee_id: r.addressee_id.parse().map_err(|e: uuid::Error| AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
        addressee_name: r.addressee_name,
        status,
        created_at: r.created_at.parse().map_err(|e: chrono::ParseError| AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
    })
}

async fn send_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<SendFriendRequestBody>,
) -> Result<Json<FriendRequest>> {
    let (user_id, _) = require_auth(&headers, &state.jwt_secret)?;

    let target = db::find_user_by_username(&state.pool, &req.username)
        .await?
        .ok_or_else(|| AppError(StatusCode::NOT_FOUND, "User not found".into()))?;

    let target_id: Uuid = target.id.parse()
        .map_err(|e: uuid::Error| AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if target_id == user_id {
        return Err(AppError(StatusCode::BAD_REQUEST, "Cannot add yourself".into()));
    }

    if db::find_friendship(&state.pool, user_id, target_id).await?.is_some() {
        return Err(AppError(StatusCode::CONFLICT, "Friend request already exists".into()));
    }

    let friendship_id = Uuid::new_v4();
    db::create_friend_request(&state.pool, friendship_id, user_id, target_id).await?;

    let row = db::find_friendship(&state.pool, user_id, target_id)
        .await?
        .ok_or_else(|| AppError(StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch friendship".into()))?;

    let model = friendship_to_model(row)?;
    state.hub.notify_user(target_id, common::WsMessage::FriendRequestReceived(model.clone()));

    Ok(Json(model))
}

async fn accept_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(friendship_id): Path<Uuid>,
) -> Result<Json<FriendRequest>> {
    let (user_id, _) = require_auth(&headers, &state.jwt_secret)?;

    let row = db::accept_friend_request(&state.pool, friendship_id, user_id)
        .await?
        .ok_or_else(|| AppError(StatusCode::NOT_FOUND, "Request not found".into()))?;

    let model = friendship_to_model(row)?;

    let requester_id: Uuid = model.requester_id;
    state.hub.notify_user(requester_id, common::WsMessage::FriendRequestReceived(model.clone()));

    Ok(Json(model))
}

async fn remove_friend(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(friendship_id): Path<Uuid>,
) -> Result<StatusCode> {
    let (user_id, _) = require_auth(&headers, &state.jwt_secret)?;
    db::delete_friendship(&state.pool, friendship_id, user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_friends(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<FriendsResponse>> {
    let (user_id, _) = require_auth(&headers, &state.jwt_secret)?;

    let rows = db::list_friendships(&state.pool, user_id).await?;

    let mut friends = Vec::new();
    let mut pending_incoming = Vec::new();
    let mut pending_outgoing = Vec::new();

    for row in rows {
        let addressee_id: Uuid = row.addressee_id.parse()
            .map_err(|e: uuid::Error| AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        let requester_id: Uuid = row.requester_id.parse()
            .map_err(|e: uuid::Error| AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        if row.status == "accepted" {
            let (peer_id, peer_name) = if requester_id == user_id {
                (addressee_id, row.addressee_name.clone())
            } else {
                (requester_id, row.requester_name.clone())
            };
            friends.push(FriendEntry { user_id: peer_id, username: peer_name });
        } else if requester_id == user_id {
            pending_outgoing.push(friendship_to_model(row)?);
        } else {
            pending_incoming.push(friendship_to_model(row)?);
        }
    }

    Ok(Json(FriendsResponse { friends, pending_incoming, pending_outgoing }))
}

async fn get_dm_history(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(peer_id): Path<Uuid>,
) -> Result<Json<Vec<DirectMessage>>> {
    let (user_id, _) = require_auth(&headers, &state.jwt_secret)?;

    if !db::are_friends(&state.pool, user_id, peer_id).await? {
        return Err(AppError(StatusCode::FORBIDDEN, "Not friends".into()));
    }

    let rows = db::fetch_dm_history(&state.pool, user_id, peer_id, 100).await?;
    let mut msgs: Vec<DirectMessage> = rows.into_iter().map(|r| -> Result<DirectMessage> {
        Ok(DirectMessage {
            id: r.id.parse().map_err(|e: uuid::Error| AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
            sender_id: r.sender_id.parse().map_err(|e: uuid::Error| AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
            sender_name: r.sender_name,
            receiver_id: r.receiver_id.parse().map_err(|e: uuid::Error| AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
            content: r.content,
            timestamp: r.timestamp.parse().map_err(|e: chrono::ParseError| AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
        })
    }).collect::<Result<_>>()?;
    msgs.reverse();
    Ok(Json(msgs))
}
