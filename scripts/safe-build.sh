#!/bin/bash
# safe-build.sh - Memory-limited build wrapper for Zen compiler
# Prevents OOM by using systemd-run with memory limits and timeout

set -e

# Configuration
MEMORY_LIMIT="${ZEN_BUILD_MEMORY:-10G}"      # Default 10GB limit
TIMEOUT="${ZEN_BUILD_TIMEOUT:-600}"          # Default 10 minute timeout
PROFILE="${ZEN_BUILD_PROFILE:-release}"      # Default release profile

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() { echo -e "${GREEN}[safe-build]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[safe-build]${NC} $1"; }
log_error() { echo -e "${RED}[safe-build]${NC} $1"; }

# Show current memory before build
show_memory() {
    local mem_info=$(free -h | grep Mem)
    local total=$(echo $mem_info | awk '{print $2}')
    local used=$(echo $mem_info | awk '{print $3}')
    local avail=$(echo $mem_info | awk '{print $7}')
    log_info "Memory: ${used}/${total} used, ${avail} available"
}

# Check if systemd-run is available
check_systemd() {
    if command -v systemd-run &> /dev/null; then
        return 0
    else
        log_warn "systemd-run not available, falling back to timeout only"
        return 1
    fi
}

# Build with memory limit using systemd-run
build_with_limits() {
    local cmd="$1"
    
    show_memory
    log_info "Building with memory limit: ${MEMORY_LIMIT}, timeout: ${TIMEOUT}s"
    log_info "Profile: ${PROFILE}"
    log_info "Command: ${cmd}"
    
    if check_systemd; then
        # Use systemd-run for memory limiting
        systemd-run --user --scope \
            --property=MemoryMax="${MEMORY_LIMIT}" \
            --property=MemoryHigh="$((${MEMORY_LIMIT%G} * 90 / 100))G" \
            timeout "${TIMEOUT}s" $cmd
    else
        # Fallback to timeout only
        timeout "${TIMEOUT}s" $cmd
    fi
    
    local exit_code=$?
    
    if [ $exit_code -eq 0 ]; then
        log_info "Build completed successfully"
        show_memory
    elif [ $exit_code -eq 124 ]; then
        log_error "Build timed out after ${TIMEOUT}s"
        exit 1
    elif [ $exit_code -eq 137 ]; then
        log_error "Build killed (likely OOM) - try reducing parallel jobs or increasing memory limit"
        log_warn "Suggestions:"
        log_warn "  - Use low-memory profile: ZEN_BUILD_PROFILE=low-memory $0"
        log_warn "  - Reduce codegen units: CARGO_BUILD_JOBS=1 $0"
        log_warn "  - Increase limit: ZEN_BUILD_MEMORY=12G $0"
        exit 1
    else
        log_error "Build failed with exit code: ${exit_code}"
        exit $exit_code
    fi
}

# Main command handling
case "${1:-build}" in
    build)
        if [ "$PROFILE" = "low-memory" ]; then
            build_with_limits "cargo build --profile low-memory"
        else
            build_with_limits "cargo build --release"
        fi
        ;;
    debug)
        build_with_limits "cargo build"
        ;;
    test)
        build_with_limits "cargo test --all"
        ;;
    check)
        build_with_limits "cargo check --all-targets"
        ;;
    low-memory)
        PROFILE="low-memory"
        build_with_limits "cargo build --profile low-memory"
        ;;
    *)
        # Pass through any cargo command
        build_with_limits "cargo $*"
        ;;
esac
