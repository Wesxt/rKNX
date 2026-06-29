#!/bin/bash
# Installation script for rKNX daemon service

set -e

# Colors for log output
GREEN='\033[0;32m'
NC='\033[0m' # No Color

echo -e "${GREEN}Installing rKNX daemon...${NC}"

# 1. Copy binary to /usr/local/bin
if [ -f "./rknx" ]; then
    echo "Installing binary..."
    sudo cp ./rknx /usr/local/bin/rknx
    sudo chmod +x /usr/local/bin/rknx
else
    echo "Error: rknx binary not found in current directory."
    exit 1
fi

# 2. Create configuration folder and copy sample config
sudo mkdir -p /etc/rknx
if [ -f "./config.example.toml" ]; then
    if [ ! -f "/etc/rknx/config.toml" ]; then
        echo "Creating default configuration at /etc/rknx/config.toml..."
        sudo cp ./config.example.toml /etc/rknx/config.toml
        sudo chmod 644 /etc/rknx/config.toml
    else
        echo "Configuration file /etc/rknx/config.toml already exists. Skipping overwrite."
    fi
fi

# 3. Copy systemd service file
if [ -f "./rknx.service" ]; then
    echo "Installing systemd service..."
    sudo cp ./rknx.service /etc/systemd/system/rknx.service
    sudo chmod 644 /etc/systemd/system/rknx.service
    
    echo "Reloading systemd daemon..."
    sudo systemctl daemon-reload
    
    echo "Enabling rknx service..."
    sudo systemctl enable rknx
    
    echo "Starting rknx service..."
    sudo systemctl restart rknx
    echo -e "${GREEN}rKNX service installed and started successfully!${NC}"
else
    echo "Warning: rknx.service not found. Skipping service registration."
fi
