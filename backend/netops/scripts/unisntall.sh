#!/bin/bash
set -e

echo "Stopping services..."
sudo systemctl disable kea-dhcp4-server || true
sudo systemctl disable kea-ctrl-agent || true