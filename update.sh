#!/usr/bin/env bash
set -e

REPO_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "=== RealmChat Server Update ==="

source "$HOME/.cargo/env"

cd "$REPO_DIR"
git pull

echo "Building server..."
cargo build --release -p server

echo "Restarting service..."
systemctl restart realm_chat

echo ""
echo "=== Done ==="
echo "Server updated and restarted. Check status with: journalctl -u realm_chat -f"
