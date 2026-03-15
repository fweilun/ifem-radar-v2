#!/bin/bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT_DIR"

if [ ! -f .env.prod ]; then
  echo "Missing .env.prod"
  echo "Create it from .env.prod.example first:"
  echo "  cp .env.prod.example .env.prod"
  exit 1
fi

echo "Starting production stack..."
docker compose --env-file .env.prod -f docker-compose.prod.yml up -d --build

echo "Done."
echo "Web: http://<your-server-ip>/"
echo "Health: http://<your-server-ip>/health"
echo "Create account: docker compose --env-file .env.prod -f docker-compose.prod.yml run --rm create_account <account> <password> [full_name] [role]"
