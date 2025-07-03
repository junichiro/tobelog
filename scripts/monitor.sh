#!/bin/bash

# Tobelog monitoring script
# Usage: ./scripts/monitor.sh [--alerts] [--email user@example.com] [--threshold-cpu 80] [--threshold-memory 80] [--threshold-disk 90]

set -euo pipefail

# Default configuration
SERVICE_NAME="tobelog"
DATA_DIR="/var/lib/tobelog"
LOG_DIR="/var/log/tobelog"
CONFIG_DIR="/etc/tobelog"
THRESHOLD_CPU=80
THRESHOLD_MEMORY=80
THRESHOLD_DISK=90
SEND_ALERTS=false
ALERT_EMAIL=""
DROPBOX_API_LIMIT=500  # Requests per minute

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
    echo -e "${GREEN}[OK]${NC} $1"
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
    echo "  --alerts                Enable alert notifications"
    echo "  --email EMAIL           Email address for alerts"
    echo "  --threshold-cpu PERCENT CPU usage threshold (default: $THRESHOLD_CPU%)"
    echo "  --threshold-memory PERCENT Memory usage threshold (default: $THRESHOLD_MEMORY%)"
    echo "  --threshold-disk PERCENT Disk usage threshold (default: $THRESHOLD_DISK%)"
    echo "  --help                  Show this help message"
    echo
    echo "Examples:"
    echo "  $0                                           # Basic monitoring"
    echo "  $0 --alerts --email admin@example.com       # With email alerts"
    echo "  $0 --threshold-cpu 90 --threshold-disk 95   # Custom thresholds"
    echo
}

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --alerts)
                SEND_ALERTS=true
                shift
                ;;
            --email)
                ALERT_EMAIL="$2"
                SEND_ALERTS=true
                shift 2
                ;;
            --threshold-cpu)
                THRESHOLD_CPU="$2"
                shift 2
                ;;
            --threshold-memory)
                THRESHOLD_MEMORY="$2"
                shift 2
                ;;
            --threshold-disk)
                THRESHOLD_DISK="$2"
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

# Send alert notification
send_alert() {
    local subject="$1"
    local message="$2"
    local severity="$3"
    
    if [[ "$SEND_ALERTS" == "true" ]]; then
        log_warning "ALERT: $subject"
        
        # Log alert to syslog
        logger -t "tobelog-monitor" -p user.warning "[$severity] $subject: $message"
        
        # Send email if configured
        if [[ -n "$ALERT_EMAIL" ]] && command -v mail >/dev/null 2>&1; then
            echo "$message" | mail -s "[$severity] Tobelog Alert: $subject" "$ALERT_EMAIL"
            log_info "Alert email sent to $ALERT_EMAIL"
        fi
        
        # Desktop notification if available
        if command -v notify-send >/dev/null 2>&1; then
            notify-send "Tobelog Alert" "$subject" -u critical
        fi
    fi
}

# Check service status
check_service_status() {
    echo "=== Service Status ==="
    
    if systemctl is-active "$SERVICE_NAME" >/dev/null 2>&1; then
        log_success "Service is running"
        
        # Check if service is enabled
        if systemctl is-enabled "$SERVICE_NAME" >/dev/null 2>&1; then
            log_success "Service is enabled for auto-start"
        else
            log_warning "Service is not enabled for auto-start"
        fi
        
        # Show service uptime
        local start_time
        start_time=$(systemctl show "$SERVICE_NAME" --property=ActiveEnterTimestamp --value)
        if [[ -n "$start_time" && "$start_time" != "n/a" ]]; then
            log_info "Service started: $start_time"
        fi
        
        return 0
    else
        log_error "Service is not running"
        send_alert "Service Down" "Tobelog service is not running" "CRITICAL"
        return 1
    fi
}

# Check system resources
check_system_resources() {
    echo
    echo "=== System Resources ==="
    
    # CPU usage
    local cpu_usage
    cpu_usage=$(top -bn1 | grep "Cpu(s)" | awk '{print $2}' | awk -F'%' '{print $1}')
    if (( $(echo "$cpu_usage > $THRESHOLD_CPU" | bc -l) )); then
        log_warning "High CPU usage: ${cpu_usage}%"
        send_alert "High CPU Usage" "CPU usage is ${cpu_usage}% (threshold: ${THRESHOLD_CPU}%)" "WARNING"
    else
        log_success "CPU usage: ${cpu_usage}%"
    fi
    
    # Memory usage
    local memory_info
    memory_info=$(free | grep Mem)
    local total_mem=$(echo "$memory_info" | awk '{print $2}')
    local used_mem=$(echo "$memory_info" | awk '{print $3}')
    local memory_usage=$((used_mem * 100 / total_mem))
    
    if [[ $memory_usage -gt $THRESHOLD_MEMORY ]]; then
        log_warning "High memory usage: ${memory_usage}%"
        send_alert "High Memory Usage" "Memory usage is ${memory_usage}% (threshold: ${THRESHOLD_MEMORY}%)" "WARNING"
    else
        log_success "Memory usage: ${memory_usage}%"
    fi
    
    # Disk usage for data directory
    if [[ -d "$DATA_DIR" ]]; then
        local disk_usage
        disk_usage=$(df "$DATA_DIR" | tail -1 | awk '{print $5}' | sed 's/%//')
        
        if [[ $disk_usage -gt $THRESHOLD_DISK ]]; then
            log_warning "High disk usage: ${disk_usage}%"
            send_alert "High Disk Usage" "Disk usage for $DATA_DIR is ${disk_usage}% (threshold: ${THRESHOLD_DISK}%)" "WARNING"
        else
            log_success "Disk usage ($DATA_DIR): ${disk_usage}%"
        fi
    fi
}

# Check service health endpoint
check_health_endpoint() {
    echo
    echo "=== Health Endpoint ==="
    
    local port
    port=$(systemctl show "$SERVICE_NAME" --property=Environment --value | grep SERVER_PORT | cut -d'=' -f2)
    port=${port:-3000}
    
    if curl -sf "http://localhost:$port/health" >/dev/null 2>&1; then
        log_success "Health endpoint responding"
        
        # Check response time
        local response_time
        response_time=$(curl -w "%{time_total}" -o /dev/null -s "http://localhost:$port/health")
        local response_ms=$(echo "$response_time * 1000" | bc | cut -d. -f1)
        
        if [[ $response_ms -gt 5000 ]]; then
            log_warning "Slow health endpoint response: ${response_ms}ms"
        else
            log_success "Health endpoint response time: ${response_ms}ms"
        fi
    else
        log_error "Health endpoint not responding"
        send_alert "Health Endpoint Down" "Health endpoint http://localhost:$port/health is not responding" "CRITICAL"
    fi
}

# Check log files for errors
check_log_errors() {
    echo
    echo "=== Log Analysis ==="
    
    # Check systemd journal for recent errors
    local error_count
    error_count=$(journalctl -u "$SERVICE_NAME" --since "1 hour ago" --no-pager | grep -i "error\|panic\|fatal" | wc -l)
    
    if [[ $error_count -gt 0 ]]; then
        log_warning "Found $error_count error(s) in the last hour"
        
        # Show recent errors
        echo "Recent errors:"
        journalctl -u "$SERVICE_NAME" --since "1 hour ago" --no-pager | grep -i "error\|panic\|fatal" | tail -5 | sed 's/^/  /'
        
        if [[ $error_count -gt 10 ]]; then
            send_alert "High Error Rate" "Found $error_count errors in the last hour" "WARNING"
        fi
    else
        log_success "No errors found in recent logs"
    fi
    
    # Check log file sizes
    if [[ -d "$LOG_DIR" ]]; then
        local large_logs
        large_logs=$(find "$LOG_DIR" -name "*.log" -size +100M 2>/dev/null || true)
        if [[ -n "$large_logs" ]]; then
            log_warning "Large log files detected:"
            echo "$large_logs" | while read -r logfile; do
                local size=$(du -h "$logfile" | cut -f1)
                echo "  $logfile ($size)"
            done
        fi
    fi
}

# Check database integrity
check_database() {
    echo
    echo "=== Database Health ==="
    
    local db_file="$DATA_DIR/blog.db"
    if [[ -f "$db_file" ]]; then
        # Check database accessibility
        if sqlite3 "$db_file" "SELECT 1;" >/dev/null 2>&1; then
            log_success "Database is accessible"
            
            # Check database integrity
            if sqlite3 "$db_file" "PRAGMA integrity_check;" | grep -q "ok"; then
                log_success "Database integrity check passed"
            else
                log_error "Database integrity check failed"
                send_alert "Database Corruption" "Database integrity check failed for $db_file" "CRITICAL"
            fi
            
            # Show database statistics
            local post_count
            post_count=$(sqlite3 "$db_file" "SELECT COUNT(*) FROM posts;" 2>/dev/null || echo "unknown")
            log_info "Total posts: $post_count"
            
            # Check database size
            local db_size
            db_size=$(du -h "$db_file" | cut -f1)
            log_info "Database size: $db_size"
            
        else
            log_error "Database is not accessible"
            send_alert "Database Access Failed" "Cannot access database at $db_file" "CRITICAL"
        fi
    else
        log_warning "Database file not found: $db_file"
    fi
}

# Check Dropbox API status (basic connectivity)
check_dropbox_api() {
    echo
    echo "=== Dropbox API Status ==="
    
    # Check if we can reach Dropbox API
    if curl -sf "https://api.dropboxapi.com/2/check/user" >/dev/null 2>&1; then
        log_success "Dropbox API is reachable"
    else
        log_warning "Dropbox API connectivity issues"
    fi
    
    # Check for recent API errors in logs
    local api_errors
    api_errors=$(journalctl -u "$SERVICE_NAME" --since "1 hour ago" --no-pager | grep -i "dropbox\|api" | grep -i "error\|failed" | wc -l)
    
    if [[ $api_errors -gt 0 ]]; then
        log_warning "Found $api_errors Dropbox/API error(s) in the last hour"
        if [[ $api_errors -gt 5 ]]; then
            send_alert "Dropbox API Errors" "Found $api_errors Dropbox API errors in the last hour" "WARNING"
        fi
    else
        log_success "No recent Dropbox API errors"
    fi
}

# Check file permissions
check_permissions() {
    echo
    echo "=== File Permissions ==="
    
    local issues=0
    
    # Check data directory permissions
    if [[ -d "$DATA_DIR" ]]; then
        local owner
        owner=$(stat -c '%U:%G' "$DATA_DIR")
        if [[ "$owner" == "tobelog:tobelog" ]]; then
            log_success "Data directory ownership: $owner"
        else
            log_warning "Data directory ownership: $owner (expected: tobelog:tobelog)"
            ((issues++))
        fi
    fi
    
    # Check config directory permissions
    if [[ -d "$CONFIG_DIR" ]]; then
        local config_perms
        config_perms=$(stat -c '%a' "$CONFIG_DIR")
        if [[ "$config_perms" == "755" ]]; then
            log_success "Config directory permissions: $config_perms"
        else
            log_warning "Config directory permissions: $config_perms (recommended: 755)"
        fi
        
        # Check environment file permissions
        if [[ -f "$CONFIG_DIR/environment" ]]; then
            local env_perms
            env_perms=$(stat -c '%a' "$CONFIG_DIR/environment")
            if [[ "$env_perms" == "640" ]]; then
                log_success "Environment file permissions: $env_perms"
            else
                log_warning "Environment file permissions: $env_perms (recommended: 640)"
            fi
        fi
    fi
    
    if [[ $issues -gt 0 ]]; then
        send_alert "Permission Issues" "Found $issues file permission issues" "WARNING"
    fi
}

# Generate monitoring report
generate_report() {
    echo
    echo "=== Monitoring Summary ==="
    echo
    echo "Timestamp: $(date)"
    echo "Hostname: $(hostname)"
    echo "Monitoring thresholds:"
    echo "  CPU: ${THRESHOLD_CPU}%"
    echo "  Memory: ${THRESHOLD_MEMORY}%"
    echo "  Disk: ${THRESHOLD_DISK}%"
    echo "Alert notifications: $SEND_ALERTS"
    if [[ -n "$ALERT_EMAIL" ]]; then
        echo "Alert email: $ALERT_EMAIL"
    fi
    echo
}

# Main monitoring function
main() {
    parse_args "$@"
    
    echo "=== Tobelog Service Monitoring ==="
    echo
    
    generate_report
    
    # Perform all checks
    local exit_code=0
    
    check_service_status || exit_code=1
    check_system_resources
    check_health_endpoint || exit_code=1
    check_log_errors
    check_database || exit_code=1
    check_dropbox_api
    check_permissions
    
    echo
    if [[ $exit_code -eq 0 ]]; then
        log_success "All monitoring checks passed"
    else
        log_warning "Some monitoring checks failed (exit code: $exit_code)"
    fi
    
    return $exit_code
}

# Run main function with all arguments
main "$@"