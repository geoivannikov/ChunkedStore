#!/bin/bash

GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

SERVER_URL="http://localhost:8080"

echo -e "${BLUE}=== Тестирование ChunkedStore сервера ===${NC}"
echo ""

check_result() {
    if [ $1 -eq 0 ]; then
        echo -e "${GREEN}✓ PASS${NC}"
    else
        echo -e "${RED}✗ FAIL${NC}"
    fi
    echo ""
}

echo -e "${BLUE}1. Тест Health Check${NC}"
curl -s "$SERVER_URL/healthz" | grep -q "ok"
check_result $?

echo -e "${BLUE}2. Тест PUT/GET/DELETE${NC}"
echo "PUT test.txt..."
curl -s -X PUT "$SERVER_URL/test.txt" -d "Hello World!" | grep -q "Object stored"
if [ $? -eq 0 ]; then
    echo "GET test.txt..."
    RESPONSE=$(curl -s "$SERVER_URL/test.txt")
    if [ "$RESPONSE" = "Hello World!" ]; then
        echo "DELETE test.txt..."
        curl -s -X DELETE "$SERVER_URL/test.txt" -w "%{http_code}" | grep -q "204"
        if [ $? -eq 0 ]; then
            echo "Проверяем что файл удален..."
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

echo -e "${BLUE}3. Тест вложенных путей${NC}"
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

echo -e "${BLUE}4. Тест 404 Not Found${NC}"
curl -s "$SERVER_URL/nonexistent.txt" -w "%{http_code}" | grep -q "404"
check_result $?

echo -e "${BLUE}5. Тест бинарных данных (256 байт)${NC}"
dd if=/dev/urandom of=test_binary.dat bs=256 count=1 2>/dev/null
curl -s -X PUT "$SERVER_URL/binary_test.dat" --data-binary @test_binary.dat > /dev/null
curl -s "$SERVER_URL/binary_test.dat" -o downloaded_binary.dat
diff -q test_binary.dat downloaded_binary.dat > /dev/null
DIFF_RESULT=$?
rm -f test_binary.dat downloaded_binary.dat
check_result $DIFF_RESULT

echo -e "${BLUE}6. Тест файла размером 1MB${NC}"
dd if=/dev/zero of=1mb_file.dat bs=1024 count=1024 2>/dev/null
curl -s -X PUT "$SERVER_URL/big_file.dat" --data-binary @1mb_file.dat | grep -q "Object stored"
BIG_FILE_RESULT=$?
rm -f 1mb_file.dat
check_result $BIG_FILE_RESULT

echo -e "${BLUE}7. Тест превышения лимита >1MB${NC}"
dd if=/dev/zero of=2mb_file.dat bs=1024 count=2048 2>/dev/null
curl -s -X PUT "$SERVER_URL/too_big.dat" --data-binary @2mb_file.dat -w "%{http_code}" | grep -q "400"
LIMIT_RESULT=$?
rm -f 2mb_file.dat
check_result $LIMIT_RESULT

echo -e "${BLUE}8. Стресс-тест (10 параллельных PUT)${NC}"
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
    echo "Успешно: $SUCCESS_COUNT/10"
    check_result 1
fi

echo -e "${BLUE}9. Тест DELETE несуществующего файла${NC}"
curl -s -X DELETE "$SERVER_URL/does_not_exist.txt" -w "%{http_code}" | grep -q "404"
check_result $?

echo -e "${BLUE}Очистка тестовых файлов...${NC}"
for i in {1..10}; do
    curl -s -X DELETE "$SERVER_URL/file_$i.txt" > /dev/null
done
curl -s -X DELETE "$SERVER_URL/stream1/segment001.mp4" > /dev/null
curl -s -X DELETE "$SERVER_URL/stream1/segment002.mp4" > /dev/null
curl -s -X DELETE "$SERVER_URL/stream2/manifest.mpd" > /dev/null
curl -s -X DELETE "$SERVER_URL/big_file.dat" > /dev/null

echo -e "${GREEN}Тестирование завершено!${NC}"
