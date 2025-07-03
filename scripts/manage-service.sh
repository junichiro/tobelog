#!/bin/bash

# Tobelog service management script
# Usage: ./scripts/manage-service.sh [start|stop|restart|status|logs|enable|disable|install|uninstall]

set -euo pipefail

SERVICE_NAME="tobelog"
SERVICE_FILE="/etc/systemd/system/tobelog.service"

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

# Check if service exists
check_service_exists() {
    if [[ ! -f "$SERVICE_FILE" ]]; then
        log_error "Service not installed. Run './scripts/install-systemd.sh' first."
        exit 1
    fi
}

# Check if running as root for privileged operations
check_root() {
    if [[ $EUID -ne 0 ]]; then
        log_error "This operation requires root privileges (use sudo)"
        exit 1
    fi
}

# Start service
start_service() {
    check_root
    check_service_exists
    
    log_info "Starting $SERVICE_NAME service..."
    if systemctl start "$SERVICE_NAME"; then
        log_success "Service started successfully"
        show_status
    else
        log_error "Failed to start service"
        exit 1
    fi
}

# Stop service
stop_service() {
    check_root
    check_service_exists
    
    log_info "Stopping $SERVICE_NAME service..."
    if systemctl stop "$SERVICE_NAME"; then
        log_success "Service stopped successfully"
    else
        log_error "Failed to stop service"
        exit 1
    fi
}

# Restart service
restart_service() {
    check_root
    check_service_exists
    
    log_info "Restarting $SERVICE_NAME service..."
    if systemctl restart "$SERVICE_NAME"; then
        log_success "Service restarted successfully"
        show_status
    else
        log_error "Failed to restart service"
        exit 1
    fi
}

# Show service status
show_status() {
    check_service_exists
    
    echo
    log_info "Service status:"
    systemctl status "$SERVICE_NAME" --no-pager || true
    
    echo
    log_info "Service is active:" $(systemctl is-active "$SERVICE_NAME")
    log_info "Service is enabled:" $(systemctl is-enabled "$SERVICE_NAME")
    
    if systemctl is-active "$SERVICE_NAME" >/dev/null; then
        echo
        log_info "Recent log entries:"
        journalctl -u "$SERVICE_NAME" --no-pager -n 10 || true
    fi
}

# Show logs
show_logs() {
    check_service_exists
    
    echo "Showing logs for $SERVICE_NAME service (Press Ctrl+C to exit):"
    echo
    journalctl -u "$SERVICE_NAME" -f
}

# Enable service
enable_service() {
    check_root
    check_service_exists
    
    log_info "Enabling $SERVICE_NAME service for automatic startup..."
    if systemctl enable "$SERVICE_NAME"; then
        log_success "Service enabled successfully"
    else
        log_error "Failed to enable service"
        exit 1
    fi
}

# Disable service
disable_service() {
    check_root
    check_service_exists
    
    log_info "Disabling $SERVICE_NAME service..."
    if systemctl disable "$SERVICE_NAME"; then
        log_success "Service disabled successfully"
    else
        log_error "Failed to disable service"
        exit 1
    fi
}

# Install service
install_service() {
    log_info "Installing $SERVICE_NAME service..."
    exec ./scripts/install-systemd.sh "$@"
}

# Uninstall service
uninstall_service() {
    check_root
    
    log_warning "This will completely remove the $SERVICE_NAME service and its data."
    read -p "Are you sure? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        log_info "Uninstall cancelled"
        exit 0
    fi
    
    # Stop and disable service
    if [[ -f "$SERVICE_FILE" ]]; then
        log_info "Stopping and disabling service..."
        systemctl stop "$SERVICE_NAME" 2>/dev/null || true
        systemctl disable "$SERVICE_NAME" 2>/dev/null || true
        
        # Remove service file
        rm -f "$SERVICE_FILE"
        log_success "Removed service file"
    fi
    
    # Remove configuration and data
    log_info "Removing configuration and data directories..."
    rm -rf /etc/tobelog
    rm -rf /var/lib/tobelog
    rm -rf /var/log/tobelog
    rm -rf /var/cache/tobelog
    rm -f /etc/logrotate.d/tobelog
    rm -f /etc/rsyslog.d/49-tobelog.conf
    
    # Remove user and group
    if id "tobelog" &>/dev/null; then
        userdel "tobelog" 2>/dev/null || true
        log_success "Removed user: tobelog"
    fi
    
    if getent group "tobelog" &>/dev/null; then
        groupdel "tobelog" 2>/dev/null || true
        log_success "Removed group: tobelog"
    fi
    
    # Reload systemd
    systemctl daemon-reload
    
    # Restart rsyslog if it exists
    if command -v rsyslogd >/dev/null 2>&1; then
        systemctl restart rsyslog
    fi
    
    log_success "Service uninstalled successfully"
}

# Show service info
show_info() {
    echo "=== Tobelog Service Information ==="
    echo
    echo "Service name: $SERVICE_NAME"
    echo "Service file: $SERVICE_FILE"
    echo "Configuration: /etc/tobelog/"
    echo "Data directory: /var/lib/tobelog/"
    echo "Log directory: /var/log/tobelog/"
    echo "Cache directory: /var/cache/tobelog/"
    echo
    
    if [[ -f "$SERVICE_FILE" ]]; then
        echo "Service status: $(systemctl is-active "$SERVICE_NAME" 2>/dev/null || echo "not-active")"
        echo "Auto-start: $(systemctl is-enabled "$SERVICE_NAME" 2>/dev/null || echo "not-enabled")"
    else
        echo "Service status: not-installed"
    fi
    echo
}

# Show usage information
show_usage() {
    echo "Usage: $0 [COMMAND]"
    echo
    echo "Commands:"
    echo "  start       Start the service"
    echo "  stop        Stop the service"
    echo "  restart     Restart the service"
    echo "  status      Show service status"
    echo "  logs        Show and follow service logs"
    echo "  enable      Enable service for automatic startup"
    echo "  disable     Disable automatic startup"
    echo "  install     Install the systemd service"
    echo "  uninstall   Completely remove the service and data"
    echo "  info        Show service information"
    echo "  help        Show this help message"
    echo
    echo "Examples:"
    echo "  $0 start           # Start the service"
    echo "  $0 status          # Check service status"
    echo "  $0 logs            # Follow logs"
    echo "  $0 install         # Install service"
    echo
}

# Main function
main() {
    case "${1:-help}" in
        start)
            start_service
            ;;
        stop)
            stop_service
            ;;
        restart)
            restart_service
            ;;
        status)
            show_status
            ;;
        logs)
            show_logs
            ;;
        enable)
            enable_service
            ;;
        disable)
            disable_service
            ;;
        install)
            shift
            install_service "$@"
            ;;
        uninstall)
            uninstall_service
            ;;
        info)
            show_info
            ;;
        help|--help|-h)
            show_usage
            ;;
        *)
            log_error "Unknown command: $1"
            echo
            show_usage
            exit 1
            ;;
    esac
}

# Run main function with all arguments
main "$@"