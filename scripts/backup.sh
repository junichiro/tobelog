#!/bin/bash

# Tobelog backup script
# Usage: ./scripts/backup.sh [--destination /backup/path] [--compress] [--retention 30]

set -euo pipefail

# Default configuration
DEFAULT_BACKUP_DIR="/var/backups/tobelog"
DATA_DIR="/var/lib/tobelog"
CONFIG_DIR="/etc/tobelog"
LOG_DIR="/var/log/tobelog"
SERVICE_NAME="tobelog"
RETENTION_DAYS=30
COMPRESS=false
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")

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
    echo "  --destination DIR    Backup destination directory (default: $DEFAULT_BACKUP_DIR)"
    echo "  --compress          Compress backup using gzip"
    echo "  --retention DAYS    Number of days to keep backups (default: $RETENTION_DAYS)"
    echo "  --help              Show this help message"
    echo
    echo "Examples:"
    echo "  $0                                    # Basic backup"
    echo "  $0 --destination /home/user/backups  # Custom destination"
    echo "  $0 --compress --retention 60         # Compressed with 60-day retention"
    echo
}

# Parse command line arguments
parse_args() {
    BACKUP_DIR="$DEFAULT_BACKUP_DIR"
    
    while [[ $# -gt 0 ]]; do
        case $1 in
            --destination)
                BACKUP_DIR="$2"
                shift 2
                ;;
            --compress)
                COMPRESS=true
                shift
                ;;
            --retention)
                RETENTION_DAYS="$2"
                shift 2
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
}

# Check if running as root
check_permissions() {
    if [[ ! -r "$DATA_DIR" ]] || [[ ! -r "$CONFIG_DIR" ]]; then
        log_warning "Insufficient permissions to read all directories"
        log_info "Consider running with sudo for complete backup"
    fi
}

# Create backup directory
create_backup_dir() {
    local backup_path="$BACKUP_DIR/$TIMESTAMP"
    
    if [[ ! -d "$BACKUP_DIR" ]]; then
        log_info "Creating backup directory: $BACKUP_DIR"
        mkdir -p "$BACKUP_DIR"
    fi
    
    mkdir -p "$backup_path"
    echo "$backup_path"
}

# Check service status
check_service_status() {
    if systemctl is-active "$SERVICE_NAME" >/dev/null 2>&1; then
        log_info "Service is running - backup will include live data"
        return 0
    else
        log_info "Service is not running - backup will include static data"
        return 1
    fi
}

# Backup database with consistency check
backup_database() {
    local backup_path="$1"
    local db_file="$DATA_DIR/blog.db"
    
    if [[ -f "$db_file" ]]; then
        log_info "Backing up database..."
        
        # Create database backup directory
        mkdir -p "$backup_path/database"
        
        # Check if service is running for hot backup
        if systemctl is-active "$SERVICE_NAME" >/dev/null 2>&1; then
            # Hot backup using SQLite backup API
            log_info "Performing hot backup of database..."
            sqlite3 "$db_file" ".backup '$backup_path/database/blog.db'"
        else
            # Cold backup - simple copy
            log_info "Performing cold backup of database..."
            cp "$db_file" "$backup_path/database/blog.db"
        fi
        
        # Verify database integrity
        if sqlite3 "$backup_path/database/blog.db" "PRAGMA integrity_check;" | grep -q "ok"; then
            log_success "Database backup completed and verified"
        else
            log_error "Database backup verification failed"
            return 1
        fi
        
        # Create database info
        sqlite3 "$backup_path/database/blog.db" ".schema" > "$backup_path/database/schema.sql"
        sqlite3 "$backup_path/database/blog.db" "SELECT 'Posts: ' || COUNT(*) FROM posts UNION ALL SELECT 'Categories: ' || COUNT(*) FROM categories;" > "$backup_path/database/info.txt"
        
    else
        log_warning "Database file not found: $db_file"
    fi
}

# Backup configuration files
backup_config() {
    local backup_path="$1"
    
    if [[ -d "$CONFIG_DIR" ]]; then
        log_info "Backing up configuration..."
        cp -r "$CONFIG_DIR" "$backup_path/config"
        
        # Exclude sensitive files from readable backups
        if [[ -f "$backup_path/config/environment" ]]; then
            log_warning "Environment file contains sensitive data"
            chmod 600 "$backup_path/config/environment"
        fi
        
        log_success "Configuration backup completed"
    else
        log_warning "Configuration directory not found: $CONFIG_DIR"
    fi
}

# Backup logs (recent only)
backup_logs() {
    local backup_path="$1"
    
    if [[ -d "$LOG_DIR" ]]; then
        log_info "Backing up recent logs..."
        mkdir -p "$backup_path/logs"
        
        # Copy only recent log files (last 7 days)
        find "$LOG_DIR" -name "*.log" -mtime -7 -exec cp {} "$backup_path/logs/" \; 2>/dev/null || true
        
        # Export systemd journal for last 7 days
        if command -v journalctl >/dev/null 2>&1; then
            log_info "Exporting systemd journal..."
            journalctl -u "$SERVICE_NAME" --since "7 days ago" --no-pager > "$backup_path/logs/systemd-journal.log" 2>/dev/null || true
        fi
        
        log_success "Log backup completed"
    else
        log_warning "Log directory not found: $LOG_DIR"
    fi
}

# Create backup metadata
create_metadata() {
    local backup_path="$1"
    
    log_info "Creating backup metadata..."
    
    cat > "$backup_path/backup-info.txt" << EOF
Tobelog Backup Information
=========================
Backup Date: $(date)
Backup Path: $backup_path
Hostname: $(hostname)
System: $(uname -a)

Service Status: $(systemctl is-active "$SERVICE_NAME" 2>/dev/null || echo "unknown")
Service Enabled: $(systemctl is-enabled "$SERVICE_NAME" 2>/dev/null || echo "unknown")

Backup Contents:
EOF
    
    # List backup contents
    find "$backup_path" -type f -exec ls -lh {} \; | sed 's|'$backup_path'||g' >> "$backup_path/backup-info.txt"
    
    # Add system information
    echo >> "$backup_path/backup-info.txt"
    echo "System Information:" >> "$backup_path/backup-info.txt"
    echo "==================" >> "$backup_path/backup-info.txt"
    df -h "$DATA_DIR" >> "$backup_path/backup-info.txt" 2>/dev/null || true
    echo >> "$backup_path/backup-info.txt"
    free -h >> "$backup_path/backup-info.txt" 2>/dev/null || true
}

# Compress backup if requested
compress_backup() {
    local backup_path="$1"
    
    if [[ "$COMPRESS" == "true" ]]; then
        log_info "Compressing backup..."
        local archive_name="tobelog-backup-$TIMESTAMP.tar.gz"
        local archive_path="$BACKUP_DIR/$archive_name"
        
        cd "$BACKUP_DIR"
        tar -czf "$archive_name" "$TIMESTAMP"
        
        if [[ -f "$archive_path" ]]; then
            rm -rf "$backup_path"
            log_success "Backup compressed: $archive_path"
            echo "$archive_path"
        else
            log_error "Failed to create compressed backup"
            return 1
        fi
    else
        echo "$backup_path"
    fi
}

# Clean old backups
cleanup_old_backups() {
    log_info "Cleaning up backups older than $RETENTION_DAYS days..."
    
    local deleted_count=0
    
    # Remove old directories
    find "$BACKUP_DIR" -maxdepth 1 -type d -name "20*" -mtime +$RETENTION_DAYS -exec rm -rf {} \; -exec echo "Deleted: {}" \; | while read line; do
        ((deleted_count++))
        log_info "$line"
    done
    
    # Remove old compressed files
    find "$BACKUP_DIR" -maxdepth 1 -name "tobelog-backup-*.tar.gz" -mtime +$RETENTION_DAYS -exec rm -f {} \; -exec echo "Deleted: {}" \; | while read line; do
        ((deleted_count++))
        log_info "$line"
    done
    
    if [[ $deleted_count -gt 0 ]]; then
        log_success "Cleaned up $deleted_count old backup(s)"
    else
        log_info "No old backups to clean up"
    fi
}

# Main backup function
main() {
    echo "=== Tobelog Backup Script ==="
    echo
    
    parse_args "$@"
    check_permissions
    
    log_info "Starting backup to: $BACKUP_DIR"
    log_info "Timestamp: $TIMESTAMP"
    log_info "Compression: $COMPRESS"
    log_info "Retention: $RETENTION_DAYS days"
    echo
    
    # Check service status
    check_service_status
    
    # Create backup directory
    local backup_path
    backup_path=$(create_backup_dir)
    
    # Perform backup operations
    backup_database "$backup_path"
    backup_config "$backup_path"
    backup_logs "$backup_path"
    create_metadata "$backup_path"
    
    # Compress if requested
    local final_backup_path
    final_backup_path=$(compress_backup "$backup_path")
    
    # Clean old backups
    cleanup_old_backups
    
    # Show backup summary
    echo
    log_success "=== Backup completed successfully! ==="
    echo
    echo "Backup location: $final_backup_path"
    
    if [[ -d "$final_backup_path" ]]; then
        echo "Backup size: $(du -sh "$final_backup_path" | cut -f1)"
        echo "Files created:"
        find "$final_backup_path" -type f | sed "s|$final_backup_path/|  |"
    elif [[ -f "$final_backup_path" ]]; then
        echo "Archive size: $(ls -lh "$final_backup_path" | awk '{print $5}')"
    fi
    
    echo
    echo "To restore from this backup:"
    echo "  1. Stop the service: sudo systemctl stop tobelog"
    echo "  2. Restore files from: $final_backup_path"
    echo "  3. Start the service: sudo systemctl start tobelog"
    echo
}

# Run main function with all arguments
main "$@"