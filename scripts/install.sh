#!/usr/bin/env bash
set -euo pipefail

# ── Colors ────────────────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()  { echo -e "${GREEN}[INFO]${NC}  $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
error() { echo -e "${RED}[ERROR]${NC} $*" >&2; }

# ── Root check ────────────────────────────────────────────────────────────────
if [[ $EUID -ne 0 ]]; then
    error "This script must be run as root (use sudo)."
    exit 1
fi

# ── systemd check ─────────────────────────────────────────────────────────────
if ! command -v systemctl >/dev/null 2>&1 || [[ ! -d /run/systemd/system ]]; then
    error "This system does not use systemd; cli-assistant cannot be installed by this script."
    exit 1
fi

# ── Paths (override each via environment, e.g. CLA_PREFIX) ───────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="${CLA_BIN_DIR:-/usr/local/bin}"
CONFIG_DIR="${CLA_CONFIG_DIR:-/etc/cli-assistant}"
DBUS_DIR="${CLA_DBUS_DIR:-/etc/dbus-1/system.d}"
DBUS_ACTIVATION_DIR="${CLA_DBUS_ACTIVATION_DIR:-/usr/share/dbus-1/system-services}"
SYSTEMD_DIR="${CLA_SYSTEMD_DIR:-/etc/systemd/system}"
DATA_DIR="${CLA_DATA_DIR:-/var/lib/cli-assistant}"
MAN1_DIR="${CLA_MAN1_DIR:-/usr/local/share/man/man1}"
MAN8_DIR="${CLA_MAN8_DIR:-/usr/local/share/man/man8}"

# ── Install binaries ─────────────────────────────────────────────────────────
info "Installing binaries to ${BIN_DIR} ..."
install -m 0755 "${SCRIPT_DIR}/bin/c"   "${BIN_DIR}/c"
install -m 0755 "${SCRIPT_DIR}/bin/clad" "${BIN_DIR}/clad"

# ── Install configuration ────────────────────────────────────────────────────
info "Installing configuration to ${CONFIG_DIR} ..."
mkdir -p "${CONFIG_DIR}"

if [[ ! -f "${CONFIG_DIR}/config.toml" ]]; then
    install -m 0644 "${SCRIPT_DIR}/config/config.toml" "${CONFIG_DIR}/config.toml"
    info "Default config written to ${CONFIG_DIR}/config.toml"
    warn "Edit ${CONFIG_DIR}/config.toml to set your API key and endpoint."
else
    warn "Config file already exists — skipping (not overwriting)."
fi

# ── Install D-Bus policy ─────────────────────────────────────────────────────
info "Installing D-Bus policy to ${DBUS_DIR} ..."
install -m 0644 "${SCRIPT_DIR}/config/com.cli-assistant.conf" "${DBUS_DIR}/com.cli-assistant.conf"

# Reload D-Bus to pick up the new policy
if systemctl is-active --quiet dbus; then
    systemctl reload dbus 2>/dev/null || true
    info "D-Bus daemon reloaded."
fi

# ── Install D-Bus activation services ────────────────────────────────────────
info "Installing D-Bus activation services to ${DBUS_ACTIVATION_DIR} ..."
mkdir -p "${DBUS_ACTIVATION_DIR}"
install -m 0644 "${SCRIPT_DIR}/config/com.redhat.lightspeed.chat.service" \
    "${DBUS_ACTIVATION_DIR}/com.redhat.lightspeed.chat.service"
install -m 0644 "${SCRIPT_DIR}/config/com.redhat.lightspeed.history.service" \
    "${DBUS_ACTIVATION_DIR}/com.redhat.lightspeed.history.service"
install -m 0644 "${SCRIPT_DIR}/config/com.redhat.lightspeed.user.service" \
    "${DBUS_ACTIVATION_DIR}/com.redhat.lightspeed.user.service"

if systemctl is-active --quiet dbus; then
    systemctl reload dbus 2>/dev/null || true
    info "D-Bus activation services loaded."
fi

# ── Install systemd service ──────────────────────────────────────────────────
info "Installing systemd service to ${SYSTEMD_DIR} ..."
install -m 0644 "${SCRIPT_DIR}/config/clad.service" "${SYSTEMD_DIR}/clad.service"

systemctl daemon-reload

# Create data directory for SQLite
mkdir -p "${DATA_DIR}"
chmod 0700 "${DATA_DIR}"

# ── Install man pages ────────────────────────────────────────────────────────
info "Installing man pages ..."
install -d "${MAN1_DIR}" "${MAN8_DIR}"
install -m 0644 "${SCRIPT_DIR}/data/release/man/c.1" "${MAN1_DIR}/c.1"
install -m 0644 "${SCRIPT_DIR}/data/release/man/clad.8" "${MAN8_DIR}/clad.8"

# ── Enable & start service ───────────────────────────────────────────────────
info "Enabling and starting clad.service ..."
systemctl enable clad.service
systemctl restart clad.service

if systemctl is-active --quiet clad; then
    info "clad.service is running."
else
    warn "clad.service failed to start. Check: journalctl -u clad -n 20"
fi

# ── Done ──────────────────────────────────────────────────────────────────────
echo ""
info "Installation complete!"
echo ""
echo "  Config:     ${CONFIG_DIR}/config.toml"
echo "  Binaries:   ${BIN_DIR}/c  and  ${BIN_DIR}/clad"
echo "  Database:   ${DATA_DIR}/cla.db"
echo "  Man pages:  ${MAN1_DIR}/c.1  and  ${MAN8_DIR}/clad.8"
echo "  D-Bus:      ${DBUS_ACTIVATION_DIR}/com.redhat.lightspeed.*.service"
echo "  Service:    systemctl status clad"
echo ""
echo "  Quick test: c \"Hello, world!\""
echo ""
