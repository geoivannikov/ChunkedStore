#!/bin/bash
SERVER_URL="http://localhost:8080"
RESULTS_DIR="reports"

echo "=== Upload Performance Tests ==="

# Small files
echo "Testing small file uploads..."
wrk -t4 -c20 -d30s -s lua/put_small.lua $SERVER_URL/ > $RESULTS_DIR/upload_small.txt

# Large files  
echo "Testing large file uploads..."
wrk -t2 -c5 -d30s -s lua/put_large.lua $SERVER_URL/ > $RESULTS_DIR/upload_large.txt

# Chunked uploads
echo "Testing chunked uploads..."
wrk -t4 -c10 -d30s -s lua/put_chunked.lua $SERVER_URL/ > $RESULTS_DIR/upload_chunked.txt

echo "Upload tests completed. Results in $RESULTS_DIR/"