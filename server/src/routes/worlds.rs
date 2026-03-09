use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use uuid::Uuid;

use common::{
    models::{ChatMessage, Location, World},
    protocol::{CreateLocationRequest, CreateWorldRequest, JoinWorldRequest},
};
use crate::{
    auth::{decode_jwt, extract_bearer},
    db,
    error::{AppError, Result},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/worlds", post(create_world).get(list_worlds))
        .route("/worlds/{id}/join", post(join_world))
        .route("/worlds/{world_id}/locations", post(create_location).get(list_locations))
        .route("/worlds/{world_id}/locations/{loc_id}/messages", get(get_messages))
}

fn require_auth(headers: &HeaderMap, secret: &str) -> Result<(Uuid, String)> {
    let token = extract_bearer(headers)
        .ok_or_else(|| AppError(StatusCode::UNAUTHORIZED, "Missing token".into()))?;
    let claims = decode_jwt(token, secret)?;
    let id = claims.sub.parse::<Uuid>()
        .map_err(|e| AppError(StatusCode::UNAUTHORIZED, e.to_string()))?;
    Ok((id, claims.username))
}

fn world_from_row(r: db::WorldRow) -> Result<World> {
    let mode = match r.character_mode.as_str() {
        "Local" => common::models::CharacterMode::Local,
        _ => common::models::CharacterMode::Universal,
    };
    Ok(World {
        id: r.id.parse().map_err(|e: uuid::Error| AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
        name: r.name,
        description: r.description,
        owner_id: r.owner_id.parse().map_err(|e: uuid::Error| AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
        theme_id: r.theme_id,
        character_mode: mode,
        invite_code: r.invite_code,
        created_at: r.created_at.parse().map_err(|e: chrono::ParseError| AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
    })
}

async fn create_world(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateWorldRequest>,
) -> Result<Json<World>> {
    let (user_id, _) = require_auth(&headers, &state.jwt_secret)?;
    let id = Uuid::new_v4();
    db::create_world(
        &state.pool, id, &req.name, &req.description,
        user_id, &req.theme_id, &req.character_mode, None,
    ).await?;
    let row = db::get_world(&state.pool, id).await?.unwrap();
    Ok(Json(world_from_row(row)?))
}

async fn list_worlds(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<World>>> {
    let (user_id, _) = require_auth(&headers, &state.jwt_secret)?;
    let rows = db::list_worlds_for_user(&state.pool, user_id).await?;
    let worlds: Result<Vec<World>> = rows.into_iter().map(world_from_row).collect();
    Ok(Json(worlds?))
}

async fn join_world(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(world_id): Path<Uuid>,
    Json(_req): Json<JoinWorldRequest>,
) -> Result<Json<World>> {
    let (user_id, _) = require_auth(&headers, &state.jwt_secret)?;
    let row = db::get_world(&state.pool, world_id)
        .await?
        .ok_or_else(|| AppError(StatusCode::NOT_FOUND, "World not found".into()))?;
    db::add_world_member(&state.pool, world_id, user_id).await?;
    Ok(Json(world_from_row(row)?))
}

async fn create_location(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(world_id): Path<Uuid>,
    Json(req): Json<CreateLocationRequest>,
) -> Result<Json<Location>> {
    let (user_id, _) = require_auth(&headers, &state.jwt_secret)?;
    if !db::is_world_member(&state.pool, world_id, user_id).await? {
        return Err(AppError(StatusCode::FORBIDDEN, "Not a member".into()));
    }
    let id = Uuid::new_v4();
    db::create_location(&state.pool, id, world_id, &req.name).await?;
    Ok(Json(Location { id, world_id, name: req.name }))
}

async fn list_locations(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(world_id): Path<Uuid>,
) -> Result<Json<Vec<Location>>> {
    let (user_id, _) = require_auth(&headers, &state.jwt_secret)?;
    if !db::is_world_member(&state.pool, world_id, user_id).await? {
        return Err(AppError(StatusCode::FORBIDDEN, "Not a member".into()));
    }
    let rows = db::list_locations(&state.pool, world_id).await?;
    let locs: Result<Vec<Location>> = rows.into_iter().map(|r| {
        Ok(Location {
            id: r.id.parse().map_err(|e: uuid::Error| AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
            world_id: r.world_id.parse().map_err(|e: uuid::Error| AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
            name: r.name,
        })
    }).collect();
    Ok(Json(locs?))
}

async fn get_messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((world_id, loc_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<ChatMessage>>> {
    let (user_id, _) = require_auth(&headers, &state.jwt_secret)?;
    if !db::is_world_member(&state.pool, world_id, user_id).await? {
        return Err(AppError(StatusCode::FORBIDDEN, "Not a member".into()));
    }
    let rows = db::fetch_messages(&state.pool, loc_id, 100).await?;
    let mut msgs: Vec<ChatMessage> = rows.into_iter().map(|r| -> Result<ChatMessage> {
        Ok(ChatMessage {
            id: r.id.parse().map_err(|e: uuid::Error| AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
            world_id: r.world_id.parse().map_err(|e: uuid::Error| AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
            location_id: r.location_id.parse().map_err(|e: uuid::Error| AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
            sender_id: r.sender_id.parse().map_err(|e: uuid::Error| AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
            sender_name: r.sender_name,
            content: r.content,
            timestamp: r.timestamp.parse().map_err(|e: chrono::ParseError| AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
        })
    }).collect::<Result<_>>()?;
    msgs.reverse();
    Ok(Json(msgs))
}
