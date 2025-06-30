#!/bin/bash

# Dropbox API Setup Script for tobelog
# This script helps developers set up Dropbox API integration

set -e

echo "🔧 Dropbox API Setup for tobelog"
echo "================================="
echo

# Check if .env file exists
if [ ! -f .env ]; then
    echo "📝 Creating .env file from .env.example..."
    cp .env.example .env
    echo "✅ .env file created"
else
    echo "ℹ️  .env file already exists"
fi

echo
echo "📋 Dropbox App Setup Instructions:"
echo "1. Go to https://www.dropbox.com/developers/apps"
echo "2. Click 'Create app'"
echo "3. Choose 'Scoped access'"
echo "4. Choose 'App folder' (recommended for blog storage)"
echo "5. Name your app (e.g., 'tobelog-blog-storage')"
echo "6. Click 'Create app'"
echo

echo "🔑 Required Permissions:"
echo "In your Dropbox app settings, enable these permissions:"
echo "- files.content.read"
echo "- files.content.write"
echo "- files.metadata.read"
echo "- files.metadata.write"
echo

echo "🎫 Access Token:"
echo "1. In your app settings, go to 'Settings' tab"
echo "2. Scroll down to 'OAuth 2' section"
echo "3. Click 'Generate access token'"
echo "4. Copy the token"
echo

# Check if DROPBOX_ACCESS_TOKEN is set
if grep -q "DROPBOX_ACCESS_TOKEN=your_token_here" .env 2>/dev/null; then
    echo "⚠️  Please update DROPBOX_ACCESS_TOKEN in .env file with your actual token"
    echo
    echo "Current .env contents:"
    echo "======================"
    cat .env
    echo "======================"
    echo
    echo "Replace 'your_token_here' with your actual Dropbox access token"
else
    echo "✅ DROPBOX_ACCESS_TOKEN appears to be configured"
fi

echo
echo "🧪 To test your setup:"
echo "cargo run --bin test_dropbox"
echo
echo "📁 Recommended Dropbox folder structure:"
echo "/BlogStorage/"
echo "├── /posts/"
echo "│   ├── /2024/"
echo "│   └── /2025/"
echo "├── /media/"
echo "│   ├── /images/"
echo "│   └── /videos/"
echo "├── /drafts/"
echo "├── /templates/"
echo "└── /config/"
echo
echo "🎉 Setup complete! Update your .env file and test the connection."