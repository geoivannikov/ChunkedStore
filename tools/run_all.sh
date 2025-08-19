#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." >/dev/null 2>&1 && pwd)"

"$PROJECT_ROOT/tools/run_lint.sh"
"$PROJECT_ROOT/tools/run_unit_tests.sh"
"$PROJECT_ROOT/tools/run_integration_tests.sh"
"$PROJECT_ROOT/tools/run_coverage.sh"

echo "All tasks completed."