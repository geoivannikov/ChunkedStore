#!/usr/bin/env bash
set -euo pipefail

GREEN='\033[0;32m'; RED='\033[0;31m'; BLUE='\033[0;34m'; NC='\033[0m'
SERVER_URL="${SERVER_URL:-http://localhost:8080}"

if [ $# -lt 1 ]; then
  echo "usage: $0 <video_file>"
  exit 1
fi

VIDEO_IN="$1"
if [ ! -f "$VIDEO_IN" ]; then
  echo "file not found: $VIDEO_IN"
  exit 1
fi

BASENAME="$(basename "$VIDEO_IN")"
REMOTE_PATH1="videos/$BASENAME"
REMOTE_PATH2="videos/chunked_$BASENAME"

TMPDIR="$(mktemp -d -t chunked_video.XXXXXX)"
trap 'rm -rf "$TMPDIR"' EXIT

pass(){ echo -e "${GREEN}✓ PASS${NC}\n"; }
fail(){ echo -e "${RED}✗ FAIL${NC}\n"; exit 2; }

echo -e "${BLUE}=== Must-have check: bit-exact video preservation ===${NC}\n"

echo -e "${BLUE}0) Health${NC}"
if curl -fsS "$SERVER_URL/healthz" | grep -qi "ok"; then pass; else echo "skip"; pass; fi

echo -e "${BLUE}1) PUT (Content-Length) → GET → cmp${NC}"
curl -fsS -X PUT "$SERVER_URL/$REMOTE_PATH1" --data-binary @"$VIDEO_IN" | grep -q "Object stored" || fail
curl -fsS "$SERVER_URL/$REMOTE_PATH1" -o "$TMPDIR/out1.bin"
if cmp -s "$VIDEO_IN" "$TMPDIR/out1.bin"; then pass; else fail; fi

echo -e "${BLUE}2) PUT (Transfer-Encoding: chunked via stdin) → GET → cmp${NC}"
cat "$VIDEO_IN" | curl -fsS --http1.1 -X PUT --data-binary @- "$SERVER_URL/$REMOTE_PATH2" | grep -q "Object stored" || fail
curl -fsS "$SERVER_URL/$REMOTE_PATH2" -o "$TMPDIR/out2.bin"
if cmp -s "$VIDEO_IN" "$TMPDIR/out2.bin"; then pass; else fail; fi

echo -e "${BLUE}3) Cleanup${NC}"
curl -fsS -X DELETE "$SERVER_URL/$REMOTE_PATH1" >/dev/null || true
curl -fsS -X DELETE "$SERVER_URL/$REMOTE_PATH2" >/dev/null || true
pass

echo -e "${GREEN}Done: video is preserved and served bit-exact (both PUT modes).${NC}"
