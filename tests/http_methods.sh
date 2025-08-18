#!/bin/bash

GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

SERVER_URL="http://localhost:8080"

echo -e "${BLUE}=== ChunkedStore Server Testing ===${NC}"
echo ""

check_result() {
    if [ $1 -eq 0 ]; then
        echo -e "${GREEN}✓ PASS${NC}"
    else
        echo -e "${RED}✗ FAIL${NC}"
    fi
    echo ""
}

echo -e "${BLUE}1. Health Check Test${NC}"
curl -s "$SERVER_URL/healthz" | grep -q "ok"
check_result $?

echo -e "${BLUE}2. PUT/GET/DELETE Test${NC}"
curl -s -X PUT "$SERVER_URL/test.txt" -d "Hello World!" | grep -q "Object stored"
if [ $? -eq 0 ]; then
    RESPONSE=$(curl -s "$SERVER_URL/test.txt")
    if [ "$RESPONSE" = "Hello World!" ]; then
        curl -s -X DELETE "$SERVER_URL/test.txt" -w "%{http_code}" | grep -q "204"
        if [ $? -eq 0 ]; then
            curl -s "$SERVER_URL/test.txt" -w "%{http_code}" | grep -q "404"
            check_result $?
        else
            check_result 1
        fi
    else
        check_result 1
    fi
else
    check_result 1
fi

echo -e "${BLUE}3. Nested Paths Test${NC}"
curl -s -X PUT "$SERVER_URL/stream1/segment001.mp4" -d "video segment 1" > /dev/null
curl -s -X PUT "$SERVER_URL/stream1/segment002.mp4" -d "video segment 2" > /dev/null
curl -s -X PUT "$SERVER_URL/stream2/manifest.mpd" -d "DASH manifest" > /dev/null

RESULT1=$(curl -s "$SERVER_URL/stream1/segment001.mp4")
RESULT2=$(curl -s "$SERVER_URL/stream1/segment002.mp4")
RESULT3=$(curl -s "$SERVER_URL/stream2/manifest.mpd")

if [ "$RESULT1" = "video segment 1" ] && [ "$RESULT2" = "video segment 2" ] && [ "$RESULT3" = "DASH manifest" ]; then
    check_result 0
else
    check_result 1
fi

echo -e "${BLUE}4. 404 Not Found Test${NC}"
curl -s "$SERVER_URL/nonexistent.txt" -w "%{http_code}" | grep -q "404"
check_result $?

echo -e "${BLUE}5. Binary Data Test (256 bytes)${NC}"
dd if=/dev/urandom of=test_binary.dat bs=256 count=1 2>/dev/null
curl -s -X PUT "$SERVER_URL/binary_test.dat" --data-binary @test_binary.dat > /dev/null
curl -s "$SERVER_URL/binary_test.dat" -o downloaded_binary.dat
diff -q test_binary.dat downloaded_binary.dat > /dev/null
DIFF_RESULT=$?
rm -f test_binary.dat downloaded_binary.dat
check_result $DIFF_RESULT

echo -e "${BLUE}6. 1MB File Test${NC}"
dd if=/dev/zero of=1mb_file.dat bs=1024 count=1024 2>/dev/null
curl -s -X PUT "$SERVER_URL/big_file.dat" --data-binary @1mb_file.dat | grep -q "Object stored"
BIG_FILE_RESULT=$?
rm -f 1mb_file.dat
check_result $BIG_FILE_RESULT

echo -e "${BLUE}7. Stress Test (10 parallel PUTs)${NC}"
for i in {1..10}; do
    curl -s -X PUT "$SERVER_URL/file_$i.txt" -d "Content $i" > /dev/null &
done
wait

SUCCESS_COUNT=0
for i in {1..10}; do
    RESPONSE=$(curl -s "$SERVER_URL/file_$i.txt")
    if [ "$RESPONSE" = "Content $i" ]; then
        SUCCESS_COUNT=$((SUCCESS_COUNT + 1))
    fi
done

if [ $SUCCESS_COUNT -eq 10 ]; then
    check_result 0
else
    check_result 1
fi

echo -e "${BLUE}8. DELETE Non-existent File Test${NC}"
curl -s -X DELETE "$SERVER_URL/does_not_exist.txt" -w "%{http_code}" | grep -q "404"
check_result $?

echo -e "${BLUE}9. CORS Preflight Test${NC}"
curl -s -X PUT "$SERVER_URL/test.txt" -d "test content" > /dev/null

OPTIONS_STATUS=$(curl -s -X OPTIONS "$SERVER_URL/test.txt" \
    -H "Origin: http://localhost:3000" \
    -H "Access-Control-Request-Method: GET" \
    -w "%{http_code}" \
    -o /dev/null)

if [ "$OPTIONS_STATUS" = "200" ]; then
    check_result 0
else
    check_result 1
fi

echo -e "${BLUE}10. Content-Type Detection Test${NC}"
curl -s -X PUT "$SERVER_URL/test.mpd" -d "DASH manifest content" > /dev/null
CONTENT_TYPE=$(curl -s -I "$SERVER_URL/test.mpd" | grep -i "content-type" | head -1)
if echo "$CONTENT_TYPE" | grep -q "application/dash+xml"; then
    MPD_RESULT=0
else
    MPD_RESULT=1
fi

curl -s -X PUT "$SERVER_URL/test.mp4" -d "video content" > /dev/null
CONTENT_TYPE=$(curl -s -I "$SERVER_URL/test.mp4" | grep -i "content-type" | head -1)
if echo "$CONTENT_TYPE" | grep -q "video/mp4"; then
    MP4_RESULT=0
else
    MP4_RESULT=1
fi

CONTENT_TYPE=$(curl -s -I "$SERVER_URL/test.txt" | grep -i "content-type" | head -1)
if echo "$CONTENT_TYPE" | grep -q "application/octet-stream"; then
    GENERIC_RESULT=0
else
    GENERIC_RESULT=1
fi

if [ $MPD_RESULT -eq 0 ] && [ $MP4_RESULT -eq 0 ] && [ $GENERIC_RESULT -eq 0 ]; then
    check_result 0
else
    check_result 1
fi

echo -e "${BLUE}11. Cache-Control Header Test${NC}"
CACHE_CONTROL=$(curl -s -I "$SERVER_URL/test.txt" | grep -i "cache-control" | head -1)
if echo "$CACHE_CONTROL" | grep -q "no-store"; then
    check_result 0
else
    check_result 1
fi

echo -e "${BLUE}12. CORS Headers Test${NC}"
CORS_HEADERS=$(curl -s -I "$SERVER_URL/test.txt" \
    -H "Origin: http://localhost:3000" | grep -i "access-control-allow-origin")

if echo "$CORS_HEADERS" | grep -q "access-control-allow-origin"; then
    check_result 0
else
    check_result 1
fi

for i in {1..10}; do
    curl -s -X DELETE "$SERVER_URL/file_$i.txt" > /dev/null
done
curl -s -X DELETE "$SERVER_URL/stream1/segment001.mp4" > /dev/null
curl -s -X DELETE "$SERVER_URL/stream1/segment002.mp4" > /dev/null
curl -s -X DELETE "$SERVER_URL/stream2/manifest.mpd" > /dev/null
curl -s -X DELETE "$SERVER_URL/big_file.dat" > /dev/null
curl -s -X DELETE "$SERVER_URL/test.mpd" > /dev/null
curl -s -X DELETE "$SERVER_URL/test.mp4" > /dev/null
curl -s -X DELETE "$SERVER_URL/test.txt" > /dev/null
