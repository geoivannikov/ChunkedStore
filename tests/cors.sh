 #!/bin/bash

GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

SERVER_URL="http://localhost:8080"

check_result() {
    if [ $1 -eq 0 ]; then
        echo -e "${GREEN}✓ PASS${NC}"
    else
        echo -e "${RED}✗ FAIL${NC}"
    fi
    echo ""
}

echo -e "${BLUE}=== Simple CORS Test ===${NC}"

echo -e "${BLUE}1. Health check${NC}"
curl -s "$SERVER_URL/healthz" > /dev/null
check_result $?

echo -e "${BLUE}2. Create test file${NC}"
curl -s -X PUT "$SERVER_URL/test.txt" -d "test content" > /dev/null
check_result $?

echo -e "${BLUE}3. Test OPTIONS request${NC}"
curl -s -X OPTIONS "$SERVER_URL/test.txt" \
    -H "Origin: http://localhost:3000" \
    -H "Access-Control-Request-Method: GET" > /dev/null
check_result $?

echo -e "${BLUE}4. Test OPTIONS status code${NC}"
STATUS=$(curl -s -X OPTIONS "$SERVER_URL/test.txt" \
    -H "Origin: http://localhost:3000" \
    -H "Access-Control-Request-Method: GET" \
    -w "%{http_code}" \
    -o /dev/null)

if [ "$STATUS" = "200" ]; then
    check_result 0
else
    check_result 1
fi