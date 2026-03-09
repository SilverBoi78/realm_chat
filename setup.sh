#!/usr/bin/env bash
set -e

REPO_DIR="$(cd "$(dirname "$0")" && pwd)"
ENV_FILE="$REPO_DIR/.env"
SERVICE_FILE="/etc/systemd/system/realm_chat.service"

echo "=== RealmChat Server Setup ==="

# Install Rust if not present
if ! command -v cargo &>/dev/null; then
    echo "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

source "$HOME/.cargo/env"

# Generate .env if it doesn't exist
if [ ! -f "$ENV_FILE" ]; then
    echo "Generating .env..."
    JWT_SECRET=$(openssl rand -hex 32)
    cat > "$ENV_FILE" <<EOF
DATABASE_URL=sqlite:$REPO_DIR/realm_chat.db
JWT_SECRET=$JWT_SECRET
SERVER_ADDR=0.0.0.0:8080
EOF
    echo ".env created at $ENV_FILE"
else
    echo ".env already exists, skipping generation."
fi

# Build release binary
echo "Building server (this may take a few minutes)..."
cd "$REPO_DIR"
cargo build --release -p server

# Create systemd service
echo "Creating systemd service..."
cat > "$SERVICE_FILE" <<EOF
[Unit]
Description=RealmChat Server
After=network.target

[Service]
ExecStart=$REPO_DIR/target/release/server
EnvironmentFile=$ENV_FILE
Restart=on-failure
WorkingDirectory=$REPO_DIR

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable --now realm_chat

# Open firewall port
if command -v ufw &>/dev/null; then
    ufw allow 8080/tcp
    ufw --force reload
fi

echo ""
echo "=== Done ==="
echo "Server is running. Check status with: journalctl -u realm_chat -f"
echo "Server address: 91.98.84.90:8080"
