#!/bin/bash
set -e

echo "Generating SQLx offline data..."
# Ensure local DB is running for this step (we assume user has it from previous steps)
# If not, this step might fail, but let's try.
cargo sqlx prepare -- --lib

echo "Building and starting Docker containers..."
docker-compose up --build -d

echo "Setup complete! Services are running."
echo "App: http://localhost:8080"

