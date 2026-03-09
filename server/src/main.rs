mod auth;
mod db;
mod error;
mod hub;
mod routes;
mod state;

use std::sync::Arc;

use axum::Router;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::EnvFilter;

use hub::Hub;
use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("server=debug".parse()?))
        .init();

    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:./realm_chat.db".into());
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "change_me_in_production".into());
    let server_addr = std::env::var("SERVER_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".into());

    let connect_opts = database_url.parse::<SqliteConnectOptions>()?
        .create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .connect_with(connect_opts)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    let hub = Arc::new(Hub::new());
    let state = AppState { pool, hub, jwt_secret };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .merge(routes::auth::router())
        .merge(routes::worlds::router())
        .merge(routes::friends::router())
        .merge(routes::ws::router())
        .layer(cors)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&server_addr).await?;
    tracing::info!("listening on {}", server_addr);
    axum::serve(listener, app).await?;

    Ok(())
}
