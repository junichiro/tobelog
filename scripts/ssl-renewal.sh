#!/bin/bash

# SSL Certificate Renewal Script for tobelog
# This script handles Let's Encrypt certificate renewal and nginx reload

set -euo pipefail

# Configuration
DOMAIN="blog.example.com"
EMAIL="admin@example.com"
WEBROOT="/var/www/html"
NGINX_SERVICE="nginx"
BLOG_SERVICE="tobelog"
LOG_FILE="/var/log/ssl-renewal.log"

# Logging function
log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" | tee -a "$LOG_FILE"
}

# Error handling
error_exit() {
    log "ERROR: $1"
    exit 1
}

# Check if running as root
if [[ $EUID -ne 0 ]]; then
    error_exit "This script must be run as root"
fi

log "Starting SSL certificate renewal process for $DOMAIN"

# Check if certbot is installed
if ! command -v certbot &> /dev/null; then
    error_exit "certbot is not installed. Please install certbot first."
fi

# Check if nginx is running
if ! systemctl is-active --quiet $NGINX_SERVICE; then
    error_exit "nginx service is not running"
fi

# Check current certificate expiry
log "Checking current certificate expiry..."
cert_file="/etc/letsencrypt/live/$DOMAIN/fullchain.pem"
if [[ -f "$cert_file" ]]; then
    expiry_date=$(openssl x509 -in "$cert_file" -noout -enddate | cut -d= -f2)
    expiry_epoch=$(date -d "$expiry_date" +%s)
    current_epoch=$(date +%s)
    days_until_expiry=$(( (expiry_epoch - current_epoch) / 86400 ))
    
    log "Current certificate expires in $days_until_expiry days"
    
    # Only renew if certificate expires within 30 days
    if [[ $days_until_expiry -gt 30 ]]; then
        log "Certificate is still valid for $days_until_expiry days. Skipping renewal."
        exit 0
    fi
else
    log "No existing certificate found. Proceeding with initial certificate acquisition."
fi

# Create webroot directory if it doesn't exist
mkdir -p "$WEBROOT"

# Test nginx configuration before renewal
log "Testing nginx configuration..."
if ! nginx -t; then
    error_exit "nginx configuration test failed"
fi

# Attempt certificate renewal
log "Attempting certificate renewal..."
if certbot renew --quiet --webroot -w "$WEBROOT" --post-hook "systemctl reload $NGINX_SERVICE"; then
    log "Certificate renewal successful"
else
    # If renewal fails, try to obtain new certificate
    log "Renewal failed. Attempting to obtain new certificate..."
    
    # Stop nginx temporarily for standalone authentication
    log "Stopping nginx for standalone authentication..."
    systemctl stop $NGINX_SERVICE
    
    # Obtain new certificate
    if certbot certonly --standalone -d "$DOMAIN" --email "$EMAIL" --agree-tos --non-interactive; then
        log "New certificate obtained successfully"
        
        # Start nginx again
        log "Starting nginx..."
        systemctl start $NGINX_SERVICE
        
        # Test nginx configuration with new certificate
        if ! nginx -t; then
            error_exit "nginx configuration test failed with new certificate"
        fi
        
        # Reload nginx to use new certificate
        systemctl reload $NGINX_SERVICE
        log "nginx reloaded with new certificate"
    else
        # Start nginx even if certificate acquisition failed
        systemctl start $NGINX_SERVICE
        error_exit "Failed to obtain new certificate"
    fi
fi

# Verify certificate is properly installed
log "Verifying certificate installation..."
if openssl s_client -connect "$DOMAIN:443" -servername "$DOMAIN" </dev/null 2>/dev/null | openssl x509 -noout -dates; then
    log "Certificate verification successful"
else
    log "WARNING: Certificate verification failed"
fi

# Check nginx status
if systemctl is-active --quiet $NGINX_SERVICE; then
    log "nginx is running properly"
else
    error_exit "nginx is not running after certificate renewal"
fi

# Check blog service status
if systemctl is-active --quiet $BLOG_SERVICE; then
    log "tobelog service is running properly"
else
    log "WARNING: tobelog service is not running"
fi

# Log certificate expiry information
new_cert_file="/etc/letsencrypt/live/$DOMAIN/fullchain.pem"
if [[ -f "$new_cert_file" ]]; then
    new_expiry_date=$(openssl x509 -in "$new_cert_file" -noout -enddate | cut -d= -f2)
    log "New certificate expires on: $new_expiry_date"
fi

log "SSL certificate renewal process completed successfully"

# Optional: Send notification (uncomment if needed)
# echo "SSL certificate for $DOMAIN has been renewed successfully" | mail -s "SSL Certificate Renewed" admin@example.com

exit 0