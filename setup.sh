#!/bin/bash
set -e

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "Starting all services (db + backend + web) with Docker Compose..."
cd "$ROOT_DIR"
docker compose up --build -d

echo "Setup complete!"
echo "Backend health: http://localhost:8080/health"
echo "Web: http://127.0.0.1:5173"
echo "Tip: if you previously started Vite manually, stop it to avoid port confusion."

