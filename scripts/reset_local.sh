#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" != "--yes" ]]; then
  echo "This will DELETE local Docker volumes (postgres_data, minio_data) and all data."
  echo "Run: $0 --yes"
  exit 1
fi

docker compose down -v --remove-orphans

docker compose up -d

printf '\nReset complete.\n'
