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

# ── systemd check ─────────────────────────────────────────────────────────────
if ! command -v systemctl >/dev/null 2>&1 || [[ ! -d /run/systemd/system ]]; then
    error "This system does not use systemd. Remove cli-assistant with your distribution's package manager."
    exit 1
fi

# ── RPM detection ─────────────────────────────────────────────────────────────
# This script only removes a manually installed (tarball) copy. If the package
# manager owns cli-assistant, delegate to it so the rpm database stays in sync.
if command -v rpm >/dev/null 2>&1; then
    if rpm -q cli-assistant &>/dev/null || rpm -q cli-assistant-selinux &>/dev/null; then
        error "cli-assistant is installed via RPM packages. Remove them with:"
        error "    sudo dnf remove cli-assistant cli-assistant-selinux"
        error "This script only handles a manual (tarball) install."
        exit 1
    fi
fi

# ── Paths (override each via environment) ────────────────────────────────────
BIN_DIR="${CLA_BIN_DIR:-/usr/local/bin}"
CONFIG_DIR="${CLA_CONFIG_DIR:-/etc/cli-assistant}"
DBUS_DIR="${CLA_DBUS_DIR:-/etc/dbus-1/system.d}"
DBUS_ACTIVATION_DIR="${CLA_DBUS_ACTIVATION_DIR:-/usr/share/dbus-1/system-services}"
SYSTEMD_DIR="${CLA_SYSTEMD_DIR:-/etc/systemd/system}"
DATA_DIR="${CLA_DATA_DIR:-/var/lib/cli-assistant}"
MAN1_DIR="${CLA_MAN1_DIR:-/usr/local/share/man/man1}"
MAN8_DIR="${CLA_MAN8_DIR:-/usr/local/share/man/man8}"

echo ""
warn "This will remove cli-assistant from your system."
read -rp "Continue? [y/N] " confirm
if [[ "$confirm" != [yY] ]]; then
    echo "Aborted."
    exit 0
fi

# ── Stop & disable service ───────────────────────────────────────────────────
info "Stopping clad.service ..."
systemctl stop clad 2>/dev/null || true
systemctl disable clad 2>/dev/null || true

# ── Remove a file, skipping anything owned by an installed RPM package ───────
remove_file() {
    local path="$1"
    [[ -e "$path" ]] || return
    if local owner; owner="$(rpm -qf "$path" 2>/dev/null)"; then
        warn "Skipping ${path}: owned by RPM package ${owner}."
        return
    fi
    rm -f "$path"
}

info "Removing binaries ..."
remove_file "${BIN_DIR}/c"
remove_file "${BIN_DIR}/clad"

info "Removing man pages ..."
remove_file "${MAN1_DIR}/c.1"
remove_file "${MAN8_DIR}/clad.8"

info "Removing systemd service ..."
remove_file "${SYSTEMD_DIR}/clad.service"
systemctl daemon-reload 2>/dev/null || true

info "Removing D-Bus policy ..."
remove_file "${DBUS_DIR}/com.cli-assistant.conf"

info "Removing D-Bus activation services ..."
remove_file "${DBUS_ACTIVATION_DIR}/com.redhat.lightspeed.chat.service"
remove_file "${DBUS_ACTIVATION_DIR}/com.redhat.lightspeed.history.service"
remove_file "${DBUS_ACTIVATION_DIR}/com.redhat.lightspeed.user.service"

systemctl reload dbus 2>/dev/null || true

echo ""
info "Uninstallation complete."
echo ""
echo "  Config preserved: ${CONFIG_DIR}/config.toml"
echo "  Data preserved:   ${DATA_DIR}/"
echo ""
echo "  To remove all data:"
echo "    sudo rm -rf ${CONFIG_DIR} ${DATA_DIR}"
echo ""
