#!/bin/bash
SERVER_URL="http://localhost:8080"
RESULTS_DIR="reports"

echo "=== Mixed Workload Tests ==="

# Balanced workload
echo "Testing balanced workload..."
wrk -t4 -c30 -d60s -s lua/mixed_workload.lua $SERVER_URL/ > $RESULTS_DIR/mixed_balanced.txt

# Delete operations
echo "Testing delete operations..."
wrk -t2 -c10 -d30s -s lua/delete.lua $SERVER_URL/ > $RESULTS_DIR/delete_ops.txt

echo "Mixed tests completed. Results in $RESULTS_DIR/"