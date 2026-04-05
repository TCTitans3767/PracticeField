#!/bin/bash
set -e

# OS deps
echo "Installing dependencies..."

sudo apt update
sudo apt install -y kea-dhcp-server kea-ctrl-agent python3 python3-pip

# Python deps
echo "Installing Python dependencies..."

pip3 install requests

# Config files
echo "Copying config files..."

sudo cp ../config/kea-dhcp.conf.template /etc/kea/kea-dhcp4.conf
sudo cp ../config/kea-ctrl-agent.conf /etc/kea/kea-ctrl-agent.conf

# Enabling
echo "Enabling services..."

sudo systemctl enable kea-dhcp4-server
sudo systemctl enable kea-ctrl-agent

# Start
echo "Starting services..."

sudo systemctl restart kea-dhcp4-server
sudo systemctl restart kea-ctrl-agent

echo "Install complete."