#!/bin/bash
SERVER_URL="http://localhost:8080"
RESULTS_DIR="reports"

# Create results directory
mkdir -p $RESULTS_DIR

echo "=== ChunkedStore Performance Test Suite ==="
echo "Server: $SERVER_URL"
echo "Results: $RESULTS_DIR"
echo ""

# Check if server is running
if ! curl -s $SERVER_URL/healthz > /dev/null; then
    echo "Error: Server is not running at $SERVER_URL"
    echo "Please start the server first: ./tools/run_server.sh"
    exit 1
fi

echo "Server is running. Starting tests..."
echo ""

# Run all test suites
./scripts/run_upload_tests.sh
echo ""
./scripts/run_download_tests.sh  
echo ""
./scripts/run_mixed_tests.sh

echo ""
echo "=== All tests completed ==="
echo "Results saved in $RESULTS_DIR/"
echo "Run ./scripts/generate_report.sh to create HTML report"