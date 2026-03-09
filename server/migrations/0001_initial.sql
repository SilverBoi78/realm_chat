CREATE TABLE IF NOT EXISTS users (
    id          TEXT PRIMARY KEY,
    username    TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS worlds (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    description     TEXT NOT NULL DEFAULT '',
    owner_id        TEXT NOT NULL REFERENCES users(id),
    theme_id        TEXT NOT NULL DEFAULT 'fantasy',
    character_mode  TEXT NOT NULL DEFAULT 'Universal',
    invite_code     TEXT,
    created_at      TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS world_members (
    world_id    TEXT NOT NULL REFERENCES worlds(id),
    user_id     TEXT NOT NULL REFERENCES users(id),
    joined_at   TEXT NOT NULL,
    PRIMARY KEY (world_id, user_id)
);

CREATE TABLE IF NOT EXISTS locations (
    id          TEXT PRIMARY KEY,
    world_id    TEXT NOT NULL REFERENCES worlds(id),
    name        TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS messages (
    id          TEXT PRIMARY KEY,
    world_id    TEXT NOT NULL REFERENCES worlds(id),
    location_id TEXT NOT NULL REFERENCES locations(id),
    sender_id   TEXT NOT NULL REFERENCES users(id),
    content     TEXT NOT NULL,
    timestamp   TEXT NOT NULL
);
