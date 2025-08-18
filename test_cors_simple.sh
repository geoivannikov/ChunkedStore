 #!/bin/bash

GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

SERVER_URL="http://localhost:8080"

echo -e "${BLUE}=== Simple CORS Test ===${NC}"
echo ""

echo -e "${BLUE}1. Health check${NC}"
curl -s "$SERVER_URL/healthz"
echo ""

echo -e "${BLUE}2. Create test file${NC}"
curl -s -X PUT "$SERVER_URL/test.txt" -d "test content"
echo ""

echo -e "${BLUE}3. Test OPTIONS request${NC}"
echo "Command: curl -v -X OPTIONS \"$SERVER_URL/test.txt\" -H \"Origin: http://localhost:3000\""
curl -v -X OPTIONS "$SERVER_URL/test.txt" \
    -H "Origin: http://localhost:3000" \
    -H "Access-Control-Request-Method: GET" \
    2>&1

echo ""
echo -e "${BLUE}4. Test OPTIONS status code${NC}"
STATUS=$(curl -s -X OPTIONS "$SERVER_URL/test.txt" \
    -H "Origin: http://localhost:3000" \
    -H "Access-Control-Request-Method: GET" \
    -w "%{http_code}" \
    -o /dev/null)

echo "Status: $STATUS"

echo -e "${BLUE}Test completed!${NC}"