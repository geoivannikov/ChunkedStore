#!/bin/bash
RESULTS_DIR="reports"
REPORT_FILE="$RESULTS_DIR/performance_report.html"

echo "Generating performance report..."

# Function to extract metrics from wrk output
extract_metrics() {
    local file="$1"
    local test_name="$2"
    
    # Extract key metrics using grep and awk
    local rps=$(grep "Requests/sec:" "$file" | awk '{print $2}')
    local transfer=$(grep "Transfer/sec:" "$file" | awk '{print $2}')
    local latency=$(grep "Latency" "$file" | head -1 | awk '{print $2}')
    local latency_std=$(grep "Latency" "$file" | head -1 | awk '{print $3}')
    local latency_max=$(grep "Latency" "$file" | head -1 | awk '{print $4}')
    local req_sec_avg=$(grep "Req/Sec" "$file" | awk '{print $2}')
    local req_sec_std=$(grep "Req/Sec" "$file" | awk '{print $3}')
    local req_sec_max=$(grep "Req/Sec" "$file" | awk '{print $4}')
    local total_requests=$(grep "requests in" "$file" | awk '{print $1}')
    local socket_errors=$(grep "Socket errors:" "$file" | sed 's/Socket errors: //')
    local non_2xx=$(grep "Non-2xx or 3xx responses:" "$file" | awk '{print $5}')
    
    # Clean up values
    rps=${rps:-"N/A"}
    transfer=${transfer:-"N/A"}
    latency=${latency:-"N/A"}
    latency_std=${latency_std:-"N/A"}
    latency_max=${latency_max:-"N/A"}
    req_sec_avg=${req_sec_avg:-"N/A"}
    req_sec_std=${req_sec_std:-"N/A"}
    req_sec_max=${req_sec_max:-"N/A"}
    total_requests=${total_requests:-"N/A"}
    socket_errors=${socket_errors:-"N/A"}
    non_2xx=${non_2xx:-"N/A"}
    
    echo "$test_name|$rps|$transfer|$latency|$latency_std|$latency_max|$req_sec_avg|$req_sec_std|$req_sec_max|$total_requests|$socket_errors|$non_2xx"
}

cat > $REPORT_FILE << 'EOF'
<!DOCTYPE html>
<html>
<head>
    <title>ChunkedStore Performance Report</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; background: #f5f5f5; }
        .container { max-width: 1200px; margin: 0 auto; background: white; padding: 20px; border-radius: 8px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }
        h1 { color: #333; text-align: center; margin-bottom: 30px; }
        .timestamp { color: #666; font-size: 0.9em; text-align: center; margin-bottom: 30px; }
        table { width: 100%; border-collapse: collapse; margin: 20px 0; }
        th, td { padding: 12px; text-align: left; border-bottom: 1px solid #ddd; }
        th { background-color: #4CAF50; color: white; font-weight: bold; }
        tr:nth-child(even) { background-color: #f9f9f9; }
        tr:hover { background-color: #f5f5f5; }
        .metric { font-family: monospace; font-weight: bold; }
        .good { color: #4CAF50; }
        .warning { color: #FF9800; }
        .error { color: #f44336; }
        .test-section { margin: 30px 0; padding: 20px; border: 1px solid #ddd; border-radius: 5px; }
        .results { background: #f8f8f8; padding: 15px; font-family: monospace; font-size: 0.9em; white-space: pre-wrap; }
        h2 { color: #333; margin-top: 0; }
    </style>
</head>
<body>
    <div class="container">
        <h1>ChunkedStore Performance Report</h1>
        <p class="timestamp">Generated: $(date)</p>
        
        <h2>📊 Performance Summary</h2>
        <table>
            <thead>
                <tr>
                    <th>Test</th>
                    <th>RPS</th>
                    <th>Transfer/sec</th>
                    <th>Avg Latency</th>
                    <th>Max Latency</th>
                    <th>Total Requests</th>
                    <th>Socket Errors</th>
                    <th>Non-2xx Responses</th>
                </tr>
            </thead>
            <tbody>
EOF

# Generate table rows for each test
for file in $RESULTS_DIR/*.txt; do
    if [ -f "$file" ]; then
        test_name=$(basename "$file" .txt | tr '_' ' ' | sed 's/\b\w/\U&/g')
        metrics=$(extract_metrics "$file" "$test_name")
        
        IFS='|' read -r name rps transfer latency latency_std latency_max req_sec_avg req_sec_std req_sec_max total_requests socket_errors non_2xx <<< "$metrics"
        
        # Determine CSS class based on performance
        rps_class=""
        if [[ "$rps" =~ ^[0-9]+$ ]] && [ "$rps" -gt 10000 ]; then
            rps_class="good"
        elif [[ "$rps" =~ ^[0-9]+$ ]] && [ "$rps" -gt 5000 ]; then
            rps_class="warning"
        else
            rps_class="error"
        fi
        
        echo "                <tr>" >> $REPORT_FILE
        echo "                    <td><strong>$name</strong></td>" >> $REPORT_FILE
        echo "                    <td class=\"metric $rps_class\">$rps</td>" >> $REPORT_FILE
        echo "                    <td class=\"metric\">$transfer</td>" >> $REPORT_FILE
        echo "                    <td class=\"metric\">$latency</td>" >> $REPORT_FILE
        echo "                    <td class=\"metric\">$latency_max</td>" >> $REPORT_FILE
        echo "                    <td class=\"metric\">$total_requests</td>" >> $REPORT_FILE
        echo "                    <td class=\"metric\">$socket_errors</td>" >> $REPORT_FILE
        echo "                    <td class=\"metric\">$non_2xx</td>" >> $REPORT_FILE
        echo "                </tr>" >> $REPORT_FILE
    fi
done

cat >> $REPORT_FILE << 'EOF'
            </tbody>
        </table>
        
        <h2>📋 Detailed Results</h2>
EOF

# Add detailed results for each test
for file in $RESULTS_DIR/*.txt; do
    if [ -f "$file" ]; then
        test_name=$(basename "$file" .txt | tr '_' ' ' | sed 's/\b\w/\U&/g')
        echo "        <div class=\"test-section\">" >> $REPORT_FILE
        echo "            <h2>$test_name</h2>" >> $REPORT_FILE
        echo "            <div class=\"results\">" >> $REPORT_FILE
        cat "$file" >> $REPORT_FILE
        echo "            </div>" >> $REPORT_FILE
        echo "        </div>" >> $REPORT_FILE
    fi
done

cat >> $REPORT_FILE << 'EOF'
    </div>
</body>
</html>
EOF

echo "Report generated: $REPORT_FILE"
echo "Open in browser: file://$(pwd)/$REPORT_FILE"