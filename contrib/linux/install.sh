#!/bin/sh
# install.sh — guided install helper for http-smtp-rele on Linux (systemd).
#
# Run as root:
#   sudo sh contrib/linux/install.sh
#
# What it does:
#   1. Installs the binary to /usr/local/bin/
#   2. Installs the config example to /etc/http-smtp-rele.toml (if absent)
#   3. Installs the systemd unit file
#   4. Creates required directories
#   5. Enables and prompts to start the service

set -e

BINARY="${1:-./target/release/http-smtp-rele}"
UNIT_FILE="./contrib/linux/http-smtp-rele.service"
CONFIG_EXAMPLE="./examples/http-smtp-rele.toml"

echo "==> http-smtp-rele installer for Linux (systemd)"

# 1. Binary
echo "==> Installing binary to /usr/local/bin/http-smtp-rele"
install -m 755 -o root -g root "${BINARY}" /usr/local/bin/http-smtp-rele

# 2. Config
if [ ! -f /etc/http-smtp-rele.toml ]; then
    echo "==> Installing example config to /etc/http-smtp-rele.toml"
    install -m 640 -o root -g root "${CONFIG_EXAMPLE}" /etc/http-smtp-rele.toml
    echo ""
    echo "    IMPORTANT: Edit /etc/http-smtp-rele.toml before starting:"
    echo "      [mail] default_from, allowed_recipient_domains"
    echo "      [[api_keys]] id, secret"
    echo ""
else
    echo "==> /etc/http-smtp-rele.toml already exists — not overwritten"
fi

# 3. systemd unit
echo "==> Installing systemd unit to /etc/systemd/system/http-smtp-rele.service"
install -m 644 -o root -g root "${UNIT_FILE}" \
    /etc/systemd/system/http-smtp-rele.service

# 4. Directories (DynamicUser creates them, but pre-creating ensures correct ACL)
echo "==> Creating state directory /var/lib/http-smtp-rele"
install -d -m 750 /var/lib/http-smtp-rele

# 5. Reload and enable
echo "==> Reloading systemd daemon"
systemctl daemon-reload
echo "==> Enabling http-smtp-rele"
systemctl enable http-smtp-rele

echo ""
echo "==> Done. Next steps:"
echo ""
echo "    1. Edit /etc/http-smtp-rele.toml"
echo "    2. systemctl start  http-smtp-rele"
echo "    3. systemctl status http-smtp-rele"
echo ""
echo "    Config reload (no restart needed):"
echo "    systemctl reload http-smtp-rele"
echo ""
echo "    View logs:"
echo "    journalctl -u http-smtp-rele -f"
