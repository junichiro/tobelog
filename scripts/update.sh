#!/bin/bash

# Tobelog update script
# Usage: sudo ./scripts/update.sh [--binary-path /path/to/new/tobelog] [--backup] [--no-restart]

set -euo pipefail

# Configuration
SERVICE_NAME="tobelog"
BINARY_PATH="/usr/local/bin/tobelog"
CURRENT_BINARY_PATH="$BINARY_PATH"
CREATE_BACKUP=false
RESTART_SERVICE=true

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Show usage
show_usage() {
    echo "Usage: $0 [OPTIONS]"
    echo
    echo "Options:"
    echo "  --binary-path PATH   Path to new binary file (default: target/release/tobelog)"
    echo "  --backup            Create backup before update"
    echo "  --no-restart        Don't restart service after update"
    echo "  --help              Show this help message"
    echo
    echo "Examples:"
    echo "  $0                                    # Update from local build"
    echo "  $0 --backup --binary-path /tmp/tobelog  # Update with backup"
    echo "  $0 --no-restart                      # Update without restart"
    echo
}

# Parse command line arguments
parse_args() {
    local new_binary_path=""
    
    while [[ $# -gt 0 ]]; do
        case $1 in
            --binary-path)
                new_binary_path="$2"
                shift 2
                ;;
            --backup)
                CREATE_BACKUP=true
                shift
                ;;
            --no-restart)
                RESTART_SERVICE=false
                shift
                ;;
            --help)
                show_usage
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                show_usage
                exit 1
                ;;
        esac
    done
    
    # Default to local build if no path specified
    if [[ -z "$new_binary_path" ]]; then
        if [[ -f "target/release/tobelog" ]]; then
            new_binary_path="target/release/tobelog"
        else
            log_error "No binary specified and target/release/tobelog not found"
            log_info "Please build the project first: cargo build --release"
            exit 1
        fi
    fi
    
    BINARY_PATH="$new_binary_path"
}

# Check if running as root
check_root() {
    if [[ $EUID -ne 0 ]]; then
        log_error "This script must be run as root (use sudo)"
        exit 1
    fi
}

# Check if new binary exists and is valid
check_binary() {
    if [[ ! -f "$BINARY_PATH" ]]; then
        log_error "Binary not found: $BINARY_PATH"
        exit 1
    fi
    
    if [[ ! -x "$BINARY_PATH" ]]; then
        log_error "Binary is not executable: $BINARY_PATH"
        exit 1
    fi
    
    # Test binary execution
    if ! "$BINARY_PATH" --help >/dev/null 2>&1; then
        log_error "Binary appears to be invalid or corrupted: $BINARY_PATH"
        exit 1
    fi
    
    log_success "Binary validation passed: $BINARY_PATH"
}

# Check service status
check_service() {
    if ! systemctl is-active "$SERVICE_NAME" >/dev/null 2>&1; then
        log_warning "Service $SERVICE_NAME is not running"
        return 1
    fi
    return 0
}

# Create backup if requested
create_backup() {
    if [[ "$CREATE_BACKUP" == "true" ]]; then
        log_info "Creating backup before update..."
        
        if [[ -x "./scripts/backup.sh" ]]; then
            ./scripts/backup.sh --compress
            log_success "Backup created successfully"
        else
            log_warning "Backup script not found, creating simple binary backup"
            local backup_name="tobelog-$(date +%Y%m%d_%H%M%S)"
            cp "$CURRENT_BINARY_PATH" "/tmp/$backup_name"
            log_success "Binary backed up to /tmp/$backup_name"
        fi
    fi
}

# Get version information
get_version_info() {
    local binary_path="$1"
    local label="$2"
    
    echo "$label:"
    if [[ -x "$binary_path" ]]; then
        echo "  Binary: $binary_path"
        echo "  Size: $(du -h "$binary_path" | cut -f1)"
        echo "  Modified: $(stat -c '%Y' "$binary_path" | xargs -I {} date -d @{})"
        
        # Try to get version from binary if available
        local version_output
        if version_output=$("$binary_path" --version 2>/dev/null); then
            echo "  Version: $version_output"
        else
            echo "  Version: Unable to determine"
        fi
    else
        echo "  Binary not found or not executable"
    fi
    echo
}

# Stop service
stop_service() {
    if systemctl is-active "$SERVICE_NAME" >/dev/null 2>&1; then
        log_info "Stopping $SERVICE_NAME service..."
        systemctl stop "$SERVICE_NAME"
        
        # Wait for service to stop
        local timeout=30
        local elapsed=0
        while systemctl is-active "$SERVICE_NAME" >/dev/null 2>&1 && [[ $elapsed -lt $timeout ]]; do
            sleep 1
            ((elapsed++))
        done
        
        if systemctl is-active "$SERVICE_NAME" >/dev/null 2>&1; then
            log_error "Failed to stop service within $timeout seconds"
            return 1
        fi
        
        log_success "Service stopped successfully"
    else
        log_info "Service is not running"
    fi
}

# Update binary
update_binary() {
    log_info "Updating binary: $CURRENT_BINARY_PATH"
    
    # Create backup of current binary
    local backup_path="/tmp/tobelog-old-$(date +%Y%m%d_%H%M%S)"
    if [[ -f "$CURRENT_BINARY_PATH" ]]; then
        cp "$CURRENT_BINARY_PATH" "$backup_path"
        log_info "Current binary backed up to: $backup_path"
    fi
    
    # Copy new binary
    cp "$BINARY_PATH" "$CURRENT_BINARY_PATH"
    chmod +x "$CURRENT_BINARY_PATH"
    chown root:root "$CURRENT_BINARY_PATH"
    
    log_success "Binary updated successfully"
}

# Start service
start_service() {
    if [[ "$RESTART_SERVICE" == "true" ]]; then
        log_info "Starting $SERVICE_NAME service..."
        systemctl start "$SERVICE_NAME"
        
        # Wait for service to start
        local timeout=30
        local elapsed=0
        while ! systemctl is-active "$SERVICE_NAME" >/dev/null 2>&1 && [[ $elapsed -lt $timeout ]]; do
            sleep 1
            ((elapsed++))
        done
        
        if ! systemctl is-active "$SERVICE_NAME" >/dev/null 2>&1; then
            log_error "Failed to start service within $timeout seconds"
            return 1
        fi
        
        log_success "Service started successfully"
    else
        log_info "Service restart skipped (--no-restart specified)"
    fi
}

# Verify update
verify_update() {
    log_info "Verifying update..."
    
    # Check service status
    if [[ "$RESTART_SERVICE" == "true" ]]; then
        if systemctl is-active "$SERVICE_NAME" >/dev/null 2>&1; then
            log_success "Service is running"
        else
            log_error "Service is not running after update"
            return 1
        fi
        
        # Check health endpoint
        local port=3000
        local max_attempts=10
        local attempt=1
        
        log_info "Checking health endpoint (may take a moment to start up)..."
        while [[ $attempt -le $max_attempts ]]; do
            if curl -sf "http://localhost:$port/health" >/dev/null 2>&1; then
                log_success "Health endpoint responding"
                break
            else
                if [[ $attempt -eq $max_attempts ]]; then
                    log_warning "Health endpoint not responding after $max_attempts attempts"
                    log_info "Service may still be starting up"
                else
                    log_info "Attempt $attempt/$max_attempts: Health endpoint not ready, waiting..."
                    sleep 2
                fi
            fi
            ((attempt++))
        done
    fi
    
    # Show current status
    log_info "Current service status:"
    systemctl status "$SERVICE_NAME" --no-pager -l || true
}

# Rollback if update fails
rollback() {
    log_error "Update failed, attempting rollback..."
    
    local backup_files
    backup_files=$(ls -t /tmp/tobelog-old-* 2>/dev/null | head -1)
    
    if [[ -n "$backup_files" ]]; then
        log_info "Restoring from backup: $backup_files"
        cp "$backup_files" "$CURRENT_BINARY_PATH"
        chmod +x "$CURRENT_BINARY_PATH"
        
        if [[ "$RESTART_SERVICE" == "true" ]]; then
            systemctl start "$SERVICE_NAME" || true
        fi
        
        log_warning "Rollback completed. Please check service status."
    else
        log_error "No backup found for rollback"
    fi
}

# Show update summary
show_summary() {
    echo
    log_success "=== Update Summary ==="
    echo
    echo "Update completed at: $(date)"
    echo "Binary path: $CURRENT_BINARY_PATH"
    echo "Service restart: $RESTART_SERVICE"
    echo "Backup created: $CREATE_BACKUP"
    
    if [[ "$RESTART_SERVICE" == "true" ]]; then
        echo "Service status: $(systemctl is-active "$SERVICE_NAME")"
        echo "Service enabled: $(systemctl is-enabled "$SERVICE_NAME")"
    fi
    
    echo
    echo "Next steps:"
    echo "1. Check service status: sudo systemctl status $SERVICE_NAME"
    echo "2. Monitor logs: sudo journalctl -u $SERVICE_NAME -f"
    echo "3. Test application: curl http://localhost:3000/health"
    echo "4. Check monitoring: ./scripts/monitor.sh"
    echo
}

# Main update function
main() {
    echo "=== Tobelog Update Script ==="
    echo
    
    parse_args "$@"
    check_root
    check_binary
    
    log_info "Preparing to update $SERVICE_NAME..."
    echo "Current binary: $CURRENT_BINARY_PATH"
    echo "New binary: $BINARY_PATH"
    echo "Create backup: $CREATE_BACKUP"
    echo "Restart service: $RESTART_SERVICE"
    echo
    
    # Show version information
    get_version_info "$CURRENT_BINARY_PATH" "Current Version"
    get_version_info "$BINARY_PATH" "New Version"
    
    # Confirm update
    read -p "Continue with update? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        log_info "Update cancelled"
        exit 0
    fi
    
    # Store original service state
    local was_running=false
    check_service && was_running=true
    
    # Perform update
    set +e  # Allow failures for rollback
    
    create_backup
    stop_service
    
    if update_binary; then
        if start_service && verify_update; then
            show_summary
            log_success "Update completed successfully!"
        else
            log_error "Update verification failed"
            rollback
            exit 1
        fi
    else
        log_error "Binary update failed"
        rollback
        exit 1
    fi
}

# Handle script interruption
trap 'log_error "Update interrupted"; rollback; exit 1' INT TERM

# Run main function with all arguments
main "$@"