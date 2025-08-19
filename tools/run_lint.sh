 #!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." >/dev/null 2>&1 && pwd)"

echo "=== Running Lint Checks ==="

cd "$PROJECT_ROOT/chunked_store"

echo "1. Running Clippy checks..."
if cargo clippy -- -D warnings; then
    echo "✓ Clippy checks passed"
else
    echo "✗ Clippy checks failed"
    exit 1
fi

echo "2. Running cargo check..."
if cargo check; then
    echo "✓ Cargo check passed"
else
    echo "✗ Cargo check failed"
    exit 1
fi

echo "3. Running format check..."
if cargo fmt --check; then
    echo "✓ Format check passed"
else
    echo "✗ Format check failed - run 'cargo fmt' to fix"
    exit 1
fi

echo "✓ All lint checks passed!"
