use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::{SaltString, rand_core::OsRng};
use axum::{Json, Router, extract::State, http::StatusCode, routing::post};
use uuid::Uuid;

use common::protocol::{AuthResponse, LoginRequest, RegisterRequest};
use crate::{auth::encode_jwt, db, error::{AppError, Result}, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
}

async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>> {
    let trimmed = req.username.trim();
    if trimmed.is_empty() || trimmed.len() > 32 {
        return Err(AppError(StatusCode::BAD_REQUEST, "Invalid username".into()));
    }
    if req.password.len() < 6 {
        return Err(AppError(StatusCode::BAD_REQUEST, "Password too short".into()));
    }
    if db::find_user_by_username(&state.pool, trimmed).await?.is_some() {
        return Err(AppError(StatusCode::CONFLICT, "Username already taken".into()));
    }
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(req.password.as_bytes(), &salt)
        .map_err(|e| AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .to_string();
    let id = Uuid::new_v4();
    db::create_user(&state.pool, id, trimmed, &hash).await?;
    let token = encode_jwt(id, trimmed, &state.jwt_secret)?;
    Ok(Json(AuthResponse { token, user_id: id, username: trimmed.to_owned() }))
}

async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>> {
    let row = db::find_user_by_username(&state.pool, &req.username)
        .await?
        .ok_or_else(|| AppError(StatusCode::UNAUTHORIZED, "Invalid credentials".into()))?;
    let parsed_hash = PasswordHash::new(&row.password_hash)
        .map_err(|e| AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Argon2::default()
        .verify_password(req.password.as_bytes(), &parsed_hash)
        .map_err(|_| AppError(StatusCode::UNAUTHORIZED, "Invalid credentials".into()))?;
    let id: Uuid = row.id.parse().map_err(|e: uuid::Error| AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let token = encode_jwt(id, &row.username, &state.jwt_secret)?;
    Ok(Json(AuthResponse { token, user_id: id, username: row.username }))
}
