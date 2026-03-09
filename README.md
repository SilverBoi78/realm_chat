# RealmChat

A Discord-style real-time chat desktop app built in Rust, with RPG progression mechanics. Users connect to a shared server via a native desktop client. Each server is called a World, and each World has a Theme that reskins all terminology without changing the underlying mechanics.

## Technical Stack

| Layer | Technology |
|---|---|
| Client UI | Rust + egui / eframe (native desktop) |
| Server | Rust + axum (HTTP REST + WebSocket) |
| Real-time transport | WebSockets (tokio-tungstenite) |
| Auth | Username + argon2 password hash, JWT session tokens |
| Server storage | SQLite via sqlx (migrations) |
| Client cache | Local SQLite via sqlx |
| Deployment | Linux VPS (server), native binary (client) |

## Workspace Structure

```
realm_chat/
├── Cargo.toml          # workspace root (virtual manifest)
├── common/             # shared types: protocol, domain models
│   └── src/lib.rs      # WsMessage, User, World, Location, ChatMessage, etc.
├── server/             # axum HTTP + WebSocket server
│   ├── src/main.rs
│   └── migrations/     # sqlx schema migrations
└── client/             # egui/eframe desktop app
    ├── src/main.rs     # eframe::run_native entrypoint
    ├── src/app.rs      # top-level App implementing egui::App
    └── src/ws.rs       # WebSocket client on background tokio thread
```

## Environment Variables (server)

| Variable | Description |
|---|---|
| `DATABASE_URL` | SQLite path, e.g. `sqlite:./realm_chat.db` |
| `JWT_SECRET` | Secret key for signing JWT tokens |
| `SERVER_ADDR` | Bind address, e.g. `0.0.0.0:8080` |

## How to Run (local development)

```sh
# Start the server
cargo run -p server

# Start the client (separate terminal)
cargo run -p client
```

---

## Deploying the Server (Ubuntu VPS)

SSH into the server and run two commands:

```sh
git clone <your-repo-url> ~/realm_chat
~/realm_chat/setup.sh
```

That's it. The script will:
- Install Rust if not already installed
- Generate a random JWT secret and write it to `~/realm_chat/.env`
- Build the server binary in release mode
- Install and start a systemd service (`realm_chat`)
- Open port 8080 in ufw

To check server logs after setup:
```sh
journalctl -u realm_chat -f
```

---

## Client Setup (Windows)

### Prerequisites

Install Rust from [rustup.rs](https://rustup.rs). On Windows, also install the [Visual C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/).

### Run

```sh
git clone <your-repo-url>
cd realm_chat
cargo run -p client
```

The client connects to `91.98.84.90:8080` by default — no configuration needed.

---

## Usage Guide

1. Both users launch the client and register separate accounts.
2. Click the **DM** button at the bottom of the left sidebar to open the Friends panel.
3. Type a friend's username in the "Add by username" field and press **+** to send a friend request.
4. The other user sees the incoming request in their Friends panel — click **Accept**.
5. Click the friend's name to open a direct message conversation.
6. Messages are delivered in real time via WebSocket.

> **Future feature:** Worlds and group chat — one user creates a World, shares the World ID, and the other user joins it to chat in group locations.

---

## Implementation Phases

- **Phase 1** *(current)*: Cargo workspace + auth + worlds + locations + real-time chat
- **Phase 2**: Theme system (8 launch themes) + character system (XP, levels, archetypes)
- **Phase 3**: Objectives + per-world leaderboard + activity log

---

## Product Concept

RealmChat is a real-time chat + RPG desktop app where each server is called a World, and each World has a Theme. The Theme changes terminology and presentation, but not the underlying mechanics. For example, a fantasy world calls objectives “Quests,” while a cyberpunk world calls them “Contracts.” The mechanics are universal; the vocabulary is theme-driven.

Core product requirements:
1. Native desktop app in Rust
2. Real-time chat inside Worlds and Locations
3. Theme system that reskins labels without changing logic
4. Character progression with XP, levels, archetypes, titles, and stats
5. Objectives system (daily objectives, world objectives, challenge objectives, story arcs)
6. Universal vs Local character mode per world
7. MVP architecture should favor simplicity, maintainability, and a clean project structure

Technical requirements:
- Language: Rust
- App type: native desktop app
- Don't include comments

Feature requirements:

A. Authentication / Profiles
- Support a local user profile for MVP
- Store username, profile data, and progression
- Avoid overengineering auth for first version

B. Worlds
- User can create and join Worlds
- Each World has:
  - id
  - name
  - description
  - owner
  - theme_id
  - character_mode (Universal or Local)
  - invite code placeholder if multiplayer is supported

C. Locations
- Each World contains text chat locations (similar to channels)
- User can switch between locations
- Display themed naming where appropriate

D. Theme system
Implement a generic theme engine.
Store generic data internally, but map display labels through a Theme config.

Include at least these launch themes:
- Fantasy
- Cyberpunk
- Sci-Fi
- Horror
- Superhero
- Post-Apocalyptic
- Noir / Crime
- Custom

Create a ThemeVocab structure with fields like:
- world
- location
- character
- owner
- objective
- activity_log
- standing_label
- stat_names
- archetype_names
- milestone_titles

The UI must render labels dynamically based on the active World theme.

E. Character system
Each character should support:
- display_name
- archetype
- title
- xp
- level
- force
- intellect
- influence
- creation
- standing or derived ranking metric

Support two modes:
- Universal: one cross-world character that gets reskinned per theme
- Local: separate character per world

F. Progression
- XP from sending messages and completing objectives
- Level range 1–20 for MVP
- Archetype unlock at level 3
- Milestone titles at 5, 10, 15, and 20
- Four universal archetypes:
  - Forceful
  - Intellectual
  - Social
  - Creative

G. Objectives
Implement objectives with shared mechanics and themed labels.
Include:
- Daily
- World Objective
- Challenge
- Story Arc

Each objective should have:
- title
- description
- type
- reward xp
- optional stat reward
- optional expiration
- completion state

H. Chat
- Real-time-feeling chat UI
- Message list
- Message composer
- Reactions can be stubbed or implemented simply
- Typing indicator optional
- Persist messages locally

I. Leaderboard / activity log
- Per-world leaderboard
- Activity log showing objective completions, level-ups, and notable events
- All labels should be theme-aware

J. UI/UX
Design the desktop UI to feel immersive and game-like, but still usable.
Recommended layout:
- Left sidebar: Worlds
- Secondary sidebar: Locations / objectives / members
- Main panel: chat or objective board
- Right panel or modal: character sheet / leaderboard / world settings

Visual direction:
- Dark fantasy / RPG-inspired aesthetic
- Theme-aware accents per world
- Smooth panel transitions
- Clean typography
- Make it feel like a native app, not a browser page