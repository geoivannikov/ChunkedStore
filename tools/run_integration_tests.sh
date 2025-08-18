#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." >/dev/null 2>&1 && pwd)"
TESTS_DIR="$PROJECT_ROOT/tests"

if [ ! -d "$TESTS_DIR" ]; then
  echo "tests directory not found: $TESTS_DIR" >&2
  exit 1
fi

SERVER_URL="${SERVER_URL:-http://localhost:8080}"
if ! curl -fsS "$SERVER_URL/healthz" >/dev/null 2>&1; then
  echo "Server is not running; starting it in background..."
  ( cd "$PROJECT_ROOT/chunked_store" && PORT="${PORT:-8080}" cargo run ) &
  SERVER_PID=$!
  trap 'kill $SERVER_PID 2>/dev/null || true' EXIT
  for i in {1..30}; do
    if curl -fsS "$SERVER_URL/healthz" >/dev/null 2>&1; then break; fi
    sleep 0.2
  done
fi

set -x
"$TESTS_DIR/http_methods.sh"
"$TESTS_DIR/cors.sh"
"$TESTS_DIR/streaming.sh"
"$TESTS_DIR/video.sh"
set +x

echo "All integration tests finished."