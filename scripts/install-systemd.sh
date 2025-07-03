#!/bin/bash

# Tobelog systemd service installation script
# Usage: sudo ./scripts/install-systemd.sh [--binary-path /path/to/tobelog]

set -euo pipefail

# Configuration
SERVICE_USER="tobelog"
SERVICE_GROUP="tobelog"
BINARY_PATH="/usr/local/bin/tobelog"
SERVICE_FILE="/etc/systemd/system/tobelog.service"
CONFIG_DIR="/etc/tobelog"
DATA_DIR="/var/lib/tobelog"
LOG_DIR="/var/log/tobelog"
CACHE_DIR="/var/cache/tobelog"

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

# Check if running as root
check_root() {
    if [[ $EUID -ne 0 ]]; then
        log_error "This script must be run as root (use sudo)"
        exit 1
    fi
}

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --binary-path)
                BINARY_PATH="$2"
                shift 2
                ;;
            --help)
                echo "Usage: $0 [--binary-path /path/to/tobelog]"
                echo "  --binary-path    Path to the tobelog binary (default: $BINARY_PATH)"
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                exit 1
                ;;
        esac
    done
}

# Check if binary exists
check_binary() {
    if [[ ! -f "$BINARY_PATH" ]]; then
        log_error "Binary not found at $BINARY_PATH"
        log_info "Please build the binary first or specify the correct path with --binary-path"
        exit 1
    fi
    
    if [[ ! -x "$BINARY_PATH" ]]; then
        log_error "Binary at $BINARY_PATH is not executable"
        exit 1
    fi
    
    log_success "Binary found at $BINARY_PATH"
}

# Create system user and group
create_user() {
    if ! id "$SERVICE_USER" &>/dev/null; then
        log_info "Creating system user: $SERVICE_USER"
        useradd --system --no-create-home --shell /usr/sbin/nologin --group "$SERVICE_GROUP" "$SERVICE_USER" || {
            # If group creation fails, create group first
            if ! getent group "$SERVICE_GROUP" &>/dev/null; then
                groupadd --system "$SERVICE_GROUP"
            fi
            useradd --system --no-create-home --shell /usr/sbin/nologin --gid "$SERVICE_GROUP" "$SERVICE_USER"
        }
        log_success "Created system user: $SERVICE_USER"
    else
        log_info "User $SERVICE_USER already exists"
    fi
}

# Create required directories
create_directories() {
    log_info "Creating required directories..."
    
    # Create directories with proper permissions
    install -d -o root -g root -m 755 "$CONFIG_DIR"
    install -d -o "$SERVICE_USER" -g "$SERVICE_GROUP" -m 750 "$DATA_DIR"
    install -d -o "$SERVICE_USER" -g "$SERVICE_GROUP" -m 755 "$LOG_DIR"
    install -d -o "$SERVICE_USER" -g "$SERVICE_GROUP" -m 750 "$CACHE_DIR"
    
    # Create subdirectories
    install -d -o "$SERVICE_USER" -g "$SERVICE_GROUP" -m 750 "$DATA_DIR/database"
    install -d -o "$SERVICE_USER" -g "$SERVICE_GROUP" -m 755 "$LOG_DIR/archive"
    
    log_success "Created directory structure"
}

# Install systemd service file
install_service_file() {
    log_info "Installing systemd service file..."
    
    if [[ ! -f "systemd/tobelog.service" ]]; then
        log_error "Service file not found: systemd/tobelog.service"
        log_info "Please run this script from the project root directory"
        exit 1
    fi
    
    # Copy service file and update binary path
    cp "systemd/tobelog.service" "$SERVICE_FILE"
    sed -i "s|ExecStart=/usr/local/bin/tobelog|ExecStart=$BINARY_PATH|g" "$SERVICE_FILE"
    
    # Set proper permissions
    chmod 644 "$SERVICE_FILE"
    chown root:root "$SERVICE_FILE"
    
    log_success "Installed systemd service file"
}

# Create environment file template
create_environment_file() {
    log_info "Creating environment file template..."
    
    cat > "$CONFIG_DIR/environment" << 'EOF'
# Tobelog Environment Configuration
# Uncomment and set the required values

# Required: Dropbox API Access Token
# DROPBOX_ACCESS_TOKEN=your_dropbox_access_token_here

# Optional: API key for admin functions
# API_KEY=your_api_key_here

# Optional: Logging level (trace, debug, info, warn, error)
# RUST_LOG=info

# Optional: Server configuration
# SERVER_HOST=0.0.0.0
# SERVER_PORT=3000

# Optional: Database configuration
# DATABASE_URL=sqlite:///var/lib/tobelog/blog.db
EOF
    
    # Set secure permissions
    chmod 640 "$CONFIG_DIR/environment"
    chown root:"$SERVICE_GROUP" "$CONFIG_DIR/environment"
    
    log_success "Created environment file template at $CONFIG_DIR/environment"
    log_warning "Please edit $CONFIG_DIR/environment and set your Dropbox access token"
}

# Setup log rotation
setup_logrotate() {
    log_info "Setting up log rotation..."
    
    cat > "/etc/logrotate.d/tobelog" << EOF
$LOG_DIR/*.log {
    daily
    missingok
    rotate 30
    compress
    delaycompress
    notifempty
    create 644 $SERVICE_USER $SERVICE_GROUP
    postrotate
        systemctl reload-or-restart tobelog
    endscript
}
EOF
    
    log_success "Configured log rotation"
}

# Setup rsyslog configuration (optional)
setup_rsyslog() {
    if command -v rsyslogd >/dev/null 2>&1; then
        log_info "Setting up rsyslog configuration..."
        
        cat > "/etc/rsyslog.d/49-tobelog.conf" << EOF
# Tobelog logging configuration
:programname, isequal, "tobelog" $LOG_DIR/tobelog.log
& stop
EOF
        
        systemctl restart rsyslog
        log_success "Configured rsyslog for tobelog"
    else
        log_info "rsyslog not found, skipping rsyslog configuration"
    fi
}

# Reload systemd and enable service
setup_systemd() {
    log_info "Reloading systemd configuration..."
    systemctl daemon-reload
    
    log_info "Enabling tobelog service..."
    systemctl enable tobelog.service
    
    log_success "Service enabled for automatic startup"
}

# Install operational scripts
install_scripts() {
    log_info "Installing operational scripts..."
    
    # Copy scripts to system location
    local script_files=("manage-service.sh" "backup.sh" "monitor.sh" "update.sh")
    
    for script in "${script_files[@]}"; do
        if [[ -f "scripts/$script" ]]; then
            local target_name="${script/manage-service/tobelog-manage}"
            target_name="${target_name/backup/tobelog-backup}"
            target_name="${target_name/monitor/tobelog-monitor}"
            target_name="${target_name/update/tobelog-update}"
            
            cp "scripts/$script" "/usr/local/bin/$target_name"
            chmod +x "/usr/local/bin/$target_name"
            chown root:root "/usr/local/bin/$target_name"
            
            log_success "Installed: /usr/local/bin/$target_name"
        else
            log_warning "Script not found: scripts/$script"
        fi
    done
}

# Install monitoring and backup timers
install_timers() {
    log_info "Installing monitoring and backup timers..."
    
    local timer_files=("tobelog-monitor.service" "tobelog-monitor.timer" "tobelog-backup.service" "tobelog-backup.timer")
    
    for timer_file in "${timer_files[@]}"; do
        if [[ -f "systemd/$timer_file" ]]; then
            cp "systemd/$timer_file" "/etc/systemd/system/$timer_file"
            chmod 644 "/etc/systemd/system/$timer_file"
            chown root:root "/etc/systemd/system/$timer_file"
            
            log_success "Installed: /etc/systemd/system/$timer_file"
        else
            log_warning "Timer file not found: systemd/$timer_file"
        fi
    done
    
    # Enable timers
    systemctl enable tobelog-monitor.timer
    systemctl enable tobelog-backup.timer
    
    log_success "Enabled monitoring and backup timers"
}

# Display post-installation instructions
show_instructions() {
    echo
    log_success "=== Installation completed successfully! ==="
    echo
    echo "Next steps:"
    echo "1. Edit the environment file: $CONFIG_DIR/environment"
    echo "2. Set your Dropbox access token and other configuration"
    echo "3. Start the service: sudo systemctl start tobelog"
    echo "4. Check service status: sudo systemctl status tobelog"
    echo "5. View logs: sudo journalctl -u tobelog -f"
    echo
    echo "Service management commands:"
    echo "  tobelog-manage.sh start         # Start the service"
    echo "  tobelog-manage.sh stop          # Stop the service"
    echo "  tobelog-manage.sh restart       # Restart the service"
    echo "  tobelog-manage.sh status        # Check service status"
    echo "  tobelog-manage.sh logs          # View logs"
    echo
    echo "Operational scripts:"
    echo "  tobelog-backup.sh               # Manual backup"
    echo "  tobelog-monitor.sh              # Manual monitoring"
    echo "  tobelog-update.sh               # Update service"
    echo
    echo "Automatic operations:"
    echo "  systemctl status tobelog-monitor.timer   # Monitoring timer"
    echo "  systemctl status tobelog-backup.timer    # Backup timer"
    echo
    echo "Log locations:"
    echo "  systemd journal: journalctl -u tobelog"
    echo "  log files: $LOG_DIR/"
    echo
    echo "Configuration:"
    echo "  environment: $CONFIG_DIR/environment"
    echo "  service file: $SERVICE_FILE"
    echo "  documentation: SYSTEMD.md"
    echo
}

# Main installation function
main() {
    echo "=== Tobelog systemd Service Installation ==="
    echo
    
    parse_args "$@"
    check_root
    check_binary
    
    log_info "Installing tobelog systemd service..."
    echo "Binary path: $BINARY_PATH"
    echo "Service user: $SERVICE_USER"
    echo "Data directory: $DATA_DIR"
    echo "Config directory: $CONFIG_DIR"
    echo
    
    create_user
    create_directories
    install_service_file
    create_environment_file
    setup_logrotate
    setup_rsyslog
    install_scripts
    install_timers
    setup_systemd
    
    show_instructions
}

# Run main function with all arguments
main "$@"