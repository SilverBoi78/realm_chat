use chrono::Utc;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use common::models::CharacterMode;

pub async fn create_user(pool: &SqlitePool, id: Uuid, username: &str, password_hash: &str) -> anyhow::Result<()> {
    let now = Utc::now().to_rfc3339();
    sqlx::query("INSERT INTO users (id, username, password_hash, created_at) VALUES (?, ?, ?, ?)")
        .bind(id.to_string())
        .bind(username)
        .bind(password_hash)
        .bind(now)
        .execute(pool)
        .await?;
    Ok(())
}

pub struct UserRow {
    pub id: String,
    pub username: String,
    pub password_hash: String,
    pub created_at: String,
}

pub async fn find_user_by_username(pool: &SqlitePool, username: &str) -> anyhow::Result<Option<UserRow>> {
    let row = sqlx::query("SELECT id, username, password_hash, created_at FROM users WHERE username = ?")
        .bind(username)
        .fetch_optional(pool)
        .await?
        .map(|r: sqlx::sqlite::SqliteRow| UserRow {
            id: r.get("id"),
            username: r.get("username"),
            password_hash: r.get("password_hash"),
            created_at: r.get("created_at"),
        });
    Ok(row)
}

pub async fn create_world(
    pool: &SqlitePool,
    id: Uuid,
    name: &str,
    description: &str,
    owner_id: Uuid,
    theme_id: &str,
    character_mode: &CharacterMode,
    invite_code: Option<&str>,
) -> anyhow::Result<()> {
    let now = Utc::now().to_rfc3339();
    let mode = format!("{:?}", character_mode);
    sqlx::query(
        "INSERT INTO worlds (id, name, description, owner_id, theme_id, character_mode, invite_code, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(id.to_string())
    .bind(name)
    .bind(description)
    .bind(owner_id.to_string())
    .bind(theme_id)
    .bind(mode)
    .bind(invite_code)
    .bind(now)
    .execute(pool)
    .await?;
    add_world_member(pool, id, owner_id).await?;
    Ok(())
}

pub async fn add_world_member(pool: &SqlitePool, world_id: Uuid, user_id: Uuid) -> anyhow::Result<()> {
    let now = Utc::now().to_rfc3339();
    sqlx::query("INSERT OR IGNORE INTO world_members (world_id, user_id, joined_at) VALUES (?, ?, ?)")
        .bind(world_id.to_string())
        .bind(user_id.to_string())
        .bind(now)
        .execute(pool)
        .await?;
    Ok(())
}

pub struct WorldRow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub owner_id: String,
    pub theme_id: String,
    pub character_mode: String,
    pub invite_code: Option<String>,
    pub created_at: String,
}

fn row_to_world(r: &sqlx::sqlite::SqliteRow) -> WorldRow {
    WorldRow {
        id: r.get("id"),
        name: r.get("name"),
        description: r.get("description"),
        owner_id: r.get("owner_id"),
        theme_id: r.get("theme_id"),
        character_mode: r.get("character_mode"),
        invite_code: r.get("invite_code"),
        created_at: r.get("created_at"),
    }
}

pub async fn list_worlds_for_user(pool: &SqlitePool, user_id: Uuid) -> anyhow::Result<Vec<WorldRow>> {
    let rows = sqlx::query(
        "SELECT w.id, w.name, w.description, w.owner_id, w.theme_id, w.character_mode, w.invite_code, w.created_at
         FROM worlds w
         JOIN world_members wm ON wm.world_id = w.id
         WHERE wm.user_id = ?",
    )
    .bind(user_id.to_string())
    .fetch_all(pool)
    .await?
    .iter()
    .map(row_to_world)
    .collect();
    Ok(rows)
}

pub async fn get_world(pool: &SqlitePool, world_id: Uuid) -> anyhow::Result<Option<WorldRow>> {
    let row = sqlx::query(
        "SELECT id, name, description, owner_id, theme_id, character_mode, invite_code, created_at FROM worlds WHERE id = ?",
    )
    .bind(world_id.to_string())
    .fetch_optional(pool)
    .await?
    .map(|r| row_to_world(&r));
    Ok(row)
}

pub async fn is_world_member(pool: &SqlitePool, world_id: Uuid, user_id: Uuid) -> anyhow::Result<bool> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM world_members WHERE world_id = ? AND user_id = ?",
    )
    .bind(world_id.to_string())
    .bind(user_id.to_string())
    .fetch_one(pool)
    .await?;
    Ok(count > 0)
}

pub async fn create_location(pool: &SqlitePool, id: Uuid, world_id: Uuid, name: &str) -> anyhow::Result<()> {
    sqlx::query("INSERT INTO locations (id, world_id, name) VALUES (?, ?, ?)")
        .bind(id.to_string())
        .bind(world_id.to_string())
        .bind(name)
        .execute(pool)
        .await?;
    Ok(())
}

pub struct LocationRow {
    pub id: String,
    pub world_id: String,
    pub name: String,
}

pub async fn list_locations(pool: &SqlitePool, world_id: Uuid) -> anyhow::Result<Vec<LocationRow>> {
    let rows = sqlx::query("SELECT id, world_id, name FROM locations WHERE world_id = ?")
        .bind(world_id.to_string())
        .fetch_all(pool)
        .await?
        .iter()
        .map(|r: &sqlx::sqlite::SqliteRow| LocationRow {
            id: r.get("id"),
            world_id: r.get("world_id"),
            name: r.get("name"),
        })
        .collect();
    Ok(rows)
}

pub struct MessageRow {
    pub id: String,
    pub world_id: String,
    pub location_id: String,
    pub sender_id: String,
    pub sender_name: String,
    pub content: String,
    pub timestamp: String,
}

pub async fn insert_message(
    pool: &SqlitePool,
    id: Uuid,
    world_id: Uuid,
    location_id: Uuid,
    sender_id: Uuid,
    _sender_name: &str,
    content: &str,
) -> anyhow::Result<()> {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO messages (id, world_id, location_id, sender_id, content, timestamp) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(world_id.to_string())
    .bind(location_id.to_string())
    .bind(sender_id.to_string())
    .bind(content)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn fetch_messages(pool: &SqlitePool, location_id: Uuid, limit: i64) -> anyhow::Result<Vec<MessageRow>> {
    let rows = sqlx::query(
        "SELECT m.id, m.world_id, m.location_id, m.sender_id, u.username as sender_name, m.content, m.timestamp
         FROM messages m
         JOIN users u ON u.id = m.sender_id
         WHERE m.location_id = ?
         ORDER BY m.timestamp DESC
         LIMIT ?",
    )
    .bind(location_id.to_string())
    .bind(limit)
    .fetch_all(pool)
    .await?
    .iter()
    .map(|r: &sqlx::sqlite::SqliteRow| MessageRow {
        id: r.get("id"),
        world_id: r.get("world_id"),
        location_id: r.get("location_id"),
        sender_id: r.get("sender_id"),
        sender_name: r.get("sender_name"),
        content: r.get("content"),
        timestamp: r.get("timestamp"),
    })
    .collect();
    Ok(rows)
}

pub async fn get_location_world(pool: &SqlitePool, location_id: Uuid) -> anyhow::Result<Option<Uuid>> {
    let id: Option<String> = sqlx::query_scalar("SELECT world_id FROM locations WHERE id = ?")
        .bind(location_id.to_string())
        .fetch_optional(pool)
        .await?;
    Ok(id.and_then(|s| s.parse().ok()))
}

pub async fn find_user_by_id(pool: &SqlitePool, user_id: Uuid) -> anyhow::Result<Option<UserRow>> {
    let row = sqlx::query("SELECT id, username, password_hash, created_at FROM users WHERE id = ?")
        .bind(user_id.to_string())
        .fetch_optional(pool)
        .await?
        .map(|r: sqlx::sqlite::SqliteRow| UserRow {
            id: r.get("id"),
            username: r.get("username"),
            password_hash: r.get("password_hash"),
            created_at: r.get("created_at"),
        });
    Ok(row)
}

pub struct FriendshipRow {
    pub id: String,
    pub requester_id: String,
    pub requester_name: String,
    pub addressee_id: String,
    pub addressee_name: String,
    pub status: String,
    pub created_at: String,
}

fn row_to_friendship(r: &sqlx::sqlite::SqliteRow) -> FriendshipRow {
    FriendshipRow {
        id: r.get("id"),
        requester_id: r.get("requester_id"),
        requester_name: r.get("requester_name"),
        addressee_id: r.get("addressee_id"),
        addressee_name: r.get("addressee_name"),
        status: r.get("status"),
        created_at: r.get("created_at"),
    }
}

pub async fn find_friendship(pool: &SqlitePool, user_a: Uuid, user_b: Uuid) -> anyhow::Result<Option<FriendshipRow>> {
    let row = sqlx::query(
        "SELECT f.id, f.requester_id, f.addressee_id, f.status, f.created_at,
                ur.username as requester_name, ua.username as addressee_name
         FROM friendships f
         JOIN users ur ON ur.id = f.requester_id
         JOIN users ua ON ua.id = f.addressee_id
         WHERE (f.requester_id = ? AND f.addressee_id = ?)
            OR (f.requester_id = ? AND f.addressee_id = ?)",
    )
    .bind(user_a.to_string())
    .bind(user_b.to_string())
    .bind(user_b.to_string())
    .bind(user_a.to_string())
    .fetch_optional(pool)
    .await?
    .map(|r| row_to_friendship(&r));
    Ok(row)
}

pub async fn create_friend_request(pool: &SqlitePool, id: Uuid, requester_id: Uuid, addressee_id: Uuid) -> anyhow::Result<()> {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO friendships (id, requester_id, addressee_id, status, created_at) VALUES (?, ?, ?, 'pending', ?)",
    )
    .bind(id.to_string())
    .bind(requester_id.to_string())
    .bind(addressee_id.to_string())
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn accept_friend_request(pool: &SqlitePool, friendship_id: Uuid, addressee_id: Uuid) -> anyhow::Result<Option<FriendshipRow>> {
    sqlx::query(
        "UPDATE friendships SET status = 'accepted' WHERE id = ? AND addressee_id = ? AND status = 'pending'",
    )
    .bind(friendship_id.to_string())
    .bind(addressee_id.to_string())
    .execute(pool)
    .await?;

    let row = sqlx::query(
        "SELECT f.id, f.requester_id, f.addressee_id, f.status, f.created_at,
                ur.username as requester_name, ua.username as addressee_name
         FROM friendships f
         JOIN users ur ON ur.id = f.requester_id
         JOIN users ua ON ua.id = f.addressee_id
         WHERE f.id = ?",
    )
    .bind(friendship_id.to_string())
    .fetch_optional(pool)
    .await?
    .map(|r| row_to_friendship(&r));
    Ok(row)
}

pub async fn delete_friendship(pool: &SqlitePool, friendship_id: Uuid, user_id: Uuid) -> anyhow::Result<bool> {
    let result = sqlx::query(
        "DELETE FROM friendships WHERE id = ? AND (requester_id = ? OR addressee_id = ?)",
    )
    .bind(friendship_id.to_string())
    .bind(user_id.to_string())
    .bind(user_id.to_string())
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn list_friendships(pool: &SqlitePool, user_id: Uuid) -> anyhow::Result<Vec<FriendshipRow>> {
    let rows = sqlx::query(
        "SELECT f.id, f.requester_id, f.addressee_id, f.status, f.created_at,
                ur.username as requester_name, ua.username as addressee_name
         FROM friendships f
         JOIN users ur ON ur.id = f.requester_id
         JOIN users ua ON ua.id = f.addressee_id
         WHERE f.requester_id = ? OR f.addressee_id = ?",
    )
    .bind(user_id.to_string())
    .bind(user_id.to_string())
    .fetch_all(pool)
    .await?
    .iter()
    .map(|r| row_to_friendship(r))
    .collect();
    Ok(rows)
}

pub async fn are_friends(pool: &SqlitePool, user_a: Uuid, user_b: Uuid) -> anyhow::Result<bool> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM friendships
         WHERE status = 'accepted'
           AND ((requester_id = ? AND addressee_id = ?) OR (requester_id = ? AND addressee_id = ?))",
    )
    .bind(user_a.to_string())
    .bind(user_b.to_string())
    .bind(user_b.to_string())
    .bind(user_a.to_string())
    .fetch_one(pool)
    .await?;
    Ok(count > 0)
}

pub struct DmRow {
    pub id: String,
    pub sender_id: String,
    pub sender_name: String,
    pub receiver_id: String,
    pub content: String,
    pub timestamp: String,
}

pub async fn insert_dm(pool: &SqlitePool, id: Uuid, sender_id: Uuid, receiver_id: Uuid, content: &str) -> anyhow::Result<()> {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO direct_messages (id, sender_id, receiver_id, content, timestamp) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(sender_id.to_string())
    .bind(receiver_id.to_string())
    .bind(content)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn fetch_dm_history(pool: &SqlitePool, user_a: Uuid, user_b: Uuid, limit: i64) -> anyhow::Result<Vec<DmRow>> {
    let rows = sqlx::query(
        "SELECT dm.id, dm.sender_id, u.username as sender_name, dm.receiver_id, dm.content, dm.timestamp
         FROM direct_messages dm
         JOIN users u ON u.id = dm.sender_id
         WHERE (dm.sender_id = ? AND dm.receiver_id = ?)
            OR (dm.sender_id = ? AND dm.receiver_id = ?)
         ORDER BY dm.timestamp DESC
         LIMIT ?",
    )
    .bind(user_a.to_string())
    .bind(user_b.to_string())
    .bind(user_b.to_string())
    .bind(user_a.to_string())
    .bind(limit)
    .fetch_all(pool)
    .await?
    .iter()
    .map(|r: &sqlx::sqlite::SqliteRow| DmRow {
        id: r.get("id"),
        sender_id: r.get("sender_id"),
        sender_name: r.get("sender_name"),
        receiver_id: r.get("receiver_id"),
        content: r.get("content"),
        timestamp: r.get("timestamp"),
    })
    .collect();
    Ok(rows)
}
