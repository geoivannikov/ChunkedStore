#!/usr/bin/env bash
set -euo pipefail

PORT="${PORT:-8080}"
PROJECT_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." >/dev/null 2>&1 && pwd)"

cd "$PROJECT_ROOT/chunked_store"
echo "Starting server on port $PORT ..."
PORT="$PORT" cargo run