use std::sync::Arc;

use sqlx::SqlitePool;

use crate::hub::Hub;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub hub: Arc<Hub>,
    pub jwt_secret: String,
}
