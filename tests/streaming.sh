#!/usr/bin/env bash
set -u
set +m

GREEN='\033[0;32m'; RED='\033[0;31m'; BLUE='\033[0;34m'; YELLOW='\033[1;33m'; NC='\033[0m'
SERVER_URL="${SERVER_URL:-http://localhost:8080}"

CURL_BASE=(--http1.1 -sS --connect-timeout 2 --max-time 8 -H 'Expect:')

wait_with_timeout() {
  local timeout_s="$1"; shift
  local pids=("$@")
  local start now elapsed
  start=$(date +%s)
  while :; do
    local alive=0
    for pid in "${pids[@]}"; do
      if kill -0 "$pid" 2>/dev/null; then alive=1; fi
    done
    [ "$alive" -eq 0 ] && return 0
    now=$(date +%s); elapsed=$((now - start))
    if [ "$elapsed" -ge "$timeout_s" ]; then
      for pid in "${pids[@]}"; do kill -TERM "$pid" 2>/dev/null || true; done
      sleep 1
      for pid in "${pids[@]}"; do kill -KILL "$pid" 2>/dev/null || true; done
      return 124
    fi
    sleep 0.2
  done
}

echo -e "${BLUE}=== ChunkedStore Server Testing ===${NC}\n"

FAIL=0
check_result() {
  if [ "$1" -eq 0 ]; then
    echo -e "${GREEN}✓ PASS${NC}\n"
  else
    echo -e "${RED}✗ FAIL${NC}\n"
    FAIL=1
  fi
}

TMPDIR="$(mktemp -d -t chunked_store.XXXXXX)"
trap 'rm -rf "$TMPDIR"' EXIT

echo -e "${BLUE}1. Health Check${NC}"
if curl "${CURL_BASE[@]}" "$SERVER_URL/healthz" | grep -q "ok"; then
  check_result 0
else
  check_result 1
fi

echo -e "${BLUE}2. PUT/GET/DELETE (text)${NC}"
if curl "${CURL_BASE[@]}" -f -X PUT "$SERVER_URL/test.txt" --data-binary "Hello World!" >/dev/null 2>&1; then
  body="$(curl "${CURL_BASE[@]}" -f "$SERVER_URL/test.txt" 2>/dev/null || true)"
  if [ "$body" = "Hello World!" ]; then
    code="$(curl "${CURL_BASE[@]}" -o /dev/null -w '%{http_code}' -X DELETE "$SERVER_URL/test.txt" 2>/dev/null || true)"
    if [ "$code" = "204" ]; then
      code="$(curl "${CURL_BASE[@]}" -o /dev/null -w '%{http_code}' "$SERVER_URL/test.txt" 2>/dev/null || true)"
      [ "$code" = "404" ] && check_result 0 || check_result 1
    else
      check_result 1
    fi
  else
    check_result 1
  fi
else
  check_result 1
fi

echo -e "${BLUE}3. Nested paths${NC}"
curl "${CURL_BASE[@]}" -f -X PUT "$SERVER_URL/stream1/segment001.mp4" --data-binary "video segment 1" >/dev/null 2>&1 || true
curl "${CURL_BASE[@]}" -f -X PUT "$SERVER_URL/stream1/segment002.mp4" --data-binary "video segment 2" >/dev/null 2>&1 || true
curl "${CURL_BASE[@]}" -f -X PUT "$SERVER_URL/stream2/manifest.mpd"   --data-binary "DASH manifest"  >/dev/null 2>&1 || true

R1="$(curl "${CURL_BASE[@]}" "$SERVER_URL/stream1/segment001.mp4" 2>/dev/null || true)"
R2="$(curl "${CURL_BASE[@]}" "$SERVER_URL/stream1/segment002.mp4" 2>/dev/null || true)"
R3="$(curl "${CURL_BASE[@]}" "$SERVER_URL/stream2/manifest.mpd"   2>/dev/null || true)"
if [ "$R1" = "video segment 1" ] && [ "$R2" = "video segment 2" ] && [ "$R3" = "DASH manifest" ]; then
  check_result 0
else
  check_result 1
fi

echo -e "${BLUE}4. 404 Not Found${NC}"
code="$(curl "${CURL_BASE[@]}" -o /dev/null -w '%{http_code}' "$SERVER_URL/nonexistent.txt" 2>/dev/null || true)"
[ "$code" = "404" ] && check_result 0 || check_result 1

echo -e "${BLUE}5. Binary data (256 bytes)${NC}"
dd if=/dev/urandom of="$TMPDIR/test.bin" bs=256 count=1 status=none
if curl "${CURL_BASE[@]}" -f -X PUT "$SERVER_URL/binary_test.dat" --data-binary @"$TMPDIR/test.bin" >/dev/null 2>&1; then
  curl "${CURL_BASE[@]}" "$SERVER_URL/binary_test.dat" -o "$TMPDIR/dl.bin" >/dev/null 2>&1 || true
  if diff -q "$TMPDIR/test.bin" "$TMPDIR/dl.bin" >/dev/null 2>&1; then
    check_result 0
  else
    check_result 1
  fi
else
  check_result 1
fi

echo -e "${BLUE}6. Large file 1 MiB${NC}"
dd if=/dev/zero of="$TMPDIR/1mb.dat" bs=1024 count=1024 status=none
if curl "${CURL_BASE[@]}" -f -X PUT "$SERVER_URL/big_file.dat" --data-binary @"$TMPDIR/1mb.dat" >/dev/null 2>&1; then
  check_result 0
else
  check_result 1
fi

echo -e "${BLUE}7. Low-latency: availability during PUT${NC}"
FIFO="$TMPDIR/put.fifo"; mkfifo "$FIFO"
printf "Hello" > "$TMPDIR/p1"; printf "World" > "$TMPDIR/p2"

(
  curl "${CURL_BASE[@]}" -X PUT --data-binary @- "$SERVER_URL/live_stream.txt" < "$FIFO" \
    >"$TMPDIR/put.out" 2>"$TMPDIR/put.err" &
  CPID=$!
  cat "$TMPDIR/p1" > "$FIFO"
  sleep 0.8
  cat "$TMPDIR/p2" > "$FIFO"
  wait_with_timeout 8 "$CPID" 2>/dev/null || true
) &
BG_PID=$!

sleep 0.4
PARTIAL="$(curl "${CURL_BASE[@]}" --no-buffer "$SERVER_URL/live_stream.txt" | head -c 5 || true)"
[ "$PARTIAL" = "Hello" ] && check_result 0 || check_result 1

wait_with_timeout 5 "$BG_PID" 2>/dev/null || true

echo -e "${BLUE}8. Stress test (10 parallel PUTs)${NC}"
: >"$TMPDIR/put_fail"
pids=()
for i in {1..10}; do
  (
    if ! curl "${CURL_BASE[@]}" -f -X PUT "$SERVER_URL/file_$i.txt" --data-binary "Content $i" >/dev/null 2>&1; then
      echo "$i" >>"$TMPDIR/put_fail"
    fi
  ) &
  pids+=("$!")
done

if ! wait_with_timeout 12 "${pids[@]}" 2>/dev/null; then
  echo "Timeout: not all PUTs completed"
  check_result 1
else
  if [ -s "$TMPDIR/put_fail" ]; then
    check_result 1
else
    ok=0; bad=()
    for i in {1..10}; do
      body="$(curl "${CURL_BASE[@]}" "$SERVER_URL/file_$i.txt" 2>/dev/null || true)"
      if [ "$body" = "Content $i" ]; then ok=$((ok+1)); else bad+=("$i"); fi
    done
    [ $ok -eq 10 ] && check_result 0 || check_result 1
  fi
fi

echo -e "${BLUE}9. DELETE non-existent file${NC}"
code="$(curl "${CURL_BASE[@]}" -o /dev/null -w '%{http_code}' -X DELETE "$SERVER_URL/does_not_exist.txt" 2>/dev/null || true)"
[ "$code" = "404" ] && check_result 0 || check_result 1

for i in {1..10}; do curl "${CURL_BASE[@]}" -X DELETE "$SERVER_URL/file_$i.txt" >/dev/null 2>&1 || true; done
for p in stream1/segment001.mp4 stream1/segment002.mp4 stream2/manifest.mpd big_file.dat live_stream.txt binary_test.dat; do
  curl "${CURL_BASE[@]}" -X DELETE "$SERVER_URL/$p" >/dev/null 2>&1 || true
done

if [ "$FAIL" -eq 0 ]; then
  echo -e "${GREEN}Testing completed: OK${NC}"
else
  echo -e "${RED}Testing completed: errors found${NC}"
  exit 2
fi
