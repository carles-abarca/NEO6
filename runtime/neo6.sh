#!/bin/bash

# NEO6 Runtime Control Script
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ADMIN_PID_FILE="$SCRIPT_DIR/logs/neo6-admin.pid"
LOG_DIR="$SCRIPT_DIR/logs"

# Ensure log directory exists
mkdir -p "$LOG_DIR"

start_neo6() {
    echo "Starting NEO6 environment..."
    
    # Check if already running
    if [ -f "$ADMIN_PID_FILE" ] && kill -0 "$(cat "$ADMIN_PID_FILE")" 2>/dev/null; then
        echo "NEO6 Admin is already running (PID: $(cat "$ADMIN_PID_FILE"))"
        return 0
    fi
    
    # Change to runtime directory
    cd "$SCRIPT_DIR"
    
    # Set library path for dynamic loading
    export DYLD_LIBRARY_PATH="$SCRIPT_DIR/lib:$DYLD_LIBRARY_PATH"
    export LD_LIBRARY_PATH="$SCRIPT_DIR/lib:$LD_LIBRARY_PATH"
    
    # Start NEO6 Admin in background
    echo "Starting NEO6 Admin server..."
    nohup "$SCRIPT_DIR/bin/neo6-admin" --config "$SCRIPT_DIR/config/admin/admin.yaml" > "$LOG_DIR/neo6-admin.log" 2>&1 &
    ADMIN_PID=$!
    echo $ADMIN_PID > "$ADMIN_PID_FILE"
    
    # Wait a moment to ensure it started
    sleep 2
    
    if kill -0 "$ADMIN_PID" 2>/dev/null; then
        echo "NEO6 Admin started successfully (PID: $ADMIN_PID)"
        echo "Dashboard available at: http://localhost:8090"
        echo "Admin API available at: http://localhost:8090/api"
        echo "Logs in: $LOG_DIR/"
    else
        echo "Failed to start NEO6 Admin"
        rm -f "$ADMIN_PID_FILE"
        return 1
    fi
}

stop_neo6() {
    echo "Stopping NEO6 environment..."
    
    if [ -f "$ADMIN_PID_FILE" ]; then
        ADMIN_PID=$(cat "$ADMIN_PID_FILE")
        if kill -0 "$ADMIN_PID" 2>/dev/null; then
            echo "Stopping NEO6 Admin (PID: $ADMIN_PID)..."
            
            # First try graceful shutdown via API
            if command -v curl >/dev/null 2>&1; then
                echo "Attempting graceful shutdown..."
                curl -s -X POST http://localhost:8090/api/proxies/stop-all > /dev/null 2>&1 || true
                sleep 2
            fi
            
            # Send SIGTERM
            kill "$ADMIN_PID" 2>/dev/null || true
            
            # Wait for graceful shutdown
            for i in {1..10}; do
                if ! kill -0 "$ADMIN_PID" 2>/dev/null; then
                    break
                fi
                sleep 1
            done
            
            # Force kill if still running
            if kill -0 "$ADMIN_PID" 2>/dev/null; then
                echo "Force killing NEO6 Admin..."
                kill -9 "$ADMIN_PID" 2>/dev/null || true
            fi
            
            echo "NEO6 Admin stopped"
        else
            echo "NEO6 Admin was not running"
        fi
        rm -f "$ADMIN_PID_FILE"
    else
        echo "No NEO6 Admin PID file found"
    fi
    
    # Clean up any remaining processes
    pkill -f "neo6-proxy" 2>/dev/null || true
    
    echo "NEO6 environment stopped"
}

status_neo6() {
    echo "NEO6 Environment Status:"
    echo "======================="
    
    if [ -f "$ADMIN_PID_FILE" ] && kill -0 "$(cat "$ADMIN_PID_FILE")" 2>/dev/null; then
        echo "NEO6 Admin: RUNNING (PID: $(cat "$ADMIN_PID_FILE"))"
        
        # Try to get status from API
        if command -v curl >/dev/null 2>&1; then
            echo ""
            echo "API Status:"
            curl -s http://localhost:8090/api/status 2>/dev/null | head -10 || echo "API not responding"
        fi
    else
        echo "NEO6 Admin: STOPPED"
    fi
    
    echo ""
    echo "Recent logs:"
    if [ -f "$LOG_DIR/neo6-admin.log" ]; then
        tail -5 "$LOG_DIR/neo6-admin.log" || true
    else
        echo "No log file found"
    fi
}

case "$1" in
    start)
        start_neo6
        ;;
    stop)
        stop_neo6
        ;;
    restart)
        stop_neo6
        sleep 2
        start_neo6
        ;;
    status)
        status_neo6
        ;;
    *)
        echo "Usage: $0 {start|stop|restart|status}"
        echo ""
        echo "Commands:"
        echo "  start   - Start NEO6 environment (admin + auto-start proxies)"
        echo "  stop    - Stop NEO6 environment (admin + all proxies)"
        echo "  restart - Restart NEO6 environment"
        echo "  status  - Show NEO6 environment status"
        exit 1
        ;;
esac
