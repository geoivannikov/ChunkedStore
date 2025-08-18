#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." >/dev/null 2>&1 && pwd)"
cd "$PROJECT_ROOT/chunked_store"

if ! command -v cargo-tarpaulin >/dev/null 2>&1; then
  echo "cargo-tarpaulin is not installed. Installing..."
  cargo install cargo-tarpaulin
fi

mkdir -p "$PROJECT_ROOT/test_coverage"

cargo tarpaulin --out Html --output-dir "$PROJECT_ROOT/test_coverage"

echo "Coverage report generated: $PROJECT_ROOT/test_coverage/tarpaulin-report.html"