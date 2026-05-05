#!/usr/bin/env bash
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()  { echo -e "${GREEN}[INFO]${NC}  $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
error() { echo -e "${RED}[ERROR]${NC} $*" >&2; }

if [[ $EUID -ne 0 ]]; then
    error "This script must be run as root (use sudo)."
    exit 1
fi

echo ""
warn "This will remove cli-assistant from your system."
read -rp "Continue? [y/N] " confirm
if [[ "$confirm" != [yY] ]]; then
    echo "Aborted."
    exit 0
fi

# Stop & disable service
if systemctl is-active --quiet clad 2>/dev/null; then
    info "Stopping clad.service ..."
    systemctl stop clad
fi
if systemctl is-enabled --quiet clad 2>/dev/null; then
    info "Disabling clad.service ..."
    systemctl disable clad
fi

# Remove files
info "Removing binaries ..."
rm -f /usr/local/bin/c /usr/local/bin/clad

info "Removing systemd service ..."
rm -f /etc/systemd/system/clad.service
systemctl daemon-reload

info "Removing D-Bus policy ..."
rm -f /etc/dbus-1/system.d/com.cli-assistant.conf

if systemctl is-active --quiet dbus; then
    systemctl reload dbus 2>/dev/null || true
fi

echo ""
info "Uninstallation complete."
echo ""
echo "  Config preserved: /etc/cli-assistant/config.toml"
echo "  Data preserved:   /var/lib/cli-assistant/"
echo ""
echo "  To remove all data:"
echo "    sudo rm -rf /etc/cli-assistant /var/lib/cli-assistant"
echo ""
