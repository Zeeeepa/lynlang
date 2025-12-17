#!/bin/bash
# mem-watch.sh - Real-time memory monitoring during builds
# Run in a separate terminal while building

INTERVAL="${1:-2}"  # Check every N seconds
THRESHOLD="${2:-90}" # Warning threshold percentage

RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
NC='\033[0m'

echo "Memory monitor started (interval: ${INTERVAL}s, threshold: ${THRESHOLD}%)"
echo "Press Ctrl+C to stop"
echo ""

while true; do
    # Get memory stats
    read total used free <<< $(free -m | grep Mem | awk '{print $2, $3, $4}')
    percent=$((used * 100 / total))
    
    # Format timestamp
    timestamp=$(date +"%H:%M:%S")
    
    # Color based on usage
    if [ $percent -ge $THRESHOLD ]; then
        color=$RED
        status="CRITICAL"
    elif [ $percent -ge 80 ]; then
        color=$YELLOW
        status="WARNING"
    else
        color=$GREEN
        status="OK"
    fi
    
    # Find top memory consumers (cargo/rustc)
    top_procs=$(ps aux --sort=-%mem | grep -E "(cargo|rustc|cc|ld)" | head -3 | awk '{printf "  %s: %s%%\n", $11, $4}' 2>/dev/null || echo "  (none)")
    
    # Print status
    echo -e "${color}[${timestamp}] ${status}: ${used}MB / ${total}MB (${percent}%) - Free: ${free}MB${NC}"
    
    if [ $percent -ge 80 ]; then
        echo -e "${YELLOW}Top memory consumers:${NC}"
        echo "$top_procs"
    fi
    
    # Alert on critical
    if [ $percent -ge $THRESHOLD ]; then
        echo -e "${RED}⚠️  MEMORY CRITICAL - Consider killing build${NC}"
    fi
    
    sleep $INTERVAL
done
