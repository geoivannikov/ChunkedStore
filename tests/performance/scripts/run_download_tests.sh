#!/bin/bash
SERVER_URL="http://localhost:8080"
RESULTS_DIR="reports"

echo "=== Download Performance Tests ==="

# Small files
echo "Testing small file downloads..."
wrk -t4 -c50 -d30s -s lua/get_small.lua $SERVER_URL/ > $RESULTS_DIR/download_small.txt

# Large files
echo "Testing large file downloads..."
wrk -t4 -c20 -d30s -s lua/get_large.lua $SERVER_URL/ > $RESULTS_DIR/download_large.txt

# Streaming
echo "Testing streaming downloads..."
wrk -t4 -c30 -d30s -s lua/get_streaming.lua $SERVER_URL/ > $RESULTS_DIR/download_streaming.txt

# Video streaming
echo "Testing video streaming..."
wrk -t4 -c40 -d30s -s lua/video_streaming.lua $SERVER_URL/ > $RESULTS_DIR/video_streaming.txt

echo "Download tests completed. Results in $RESULTS_DIR/"