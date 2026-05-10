#!/bin/ksh
# install.sh — guided install helper for http-smtp-rele on OpenBSD.
#
# Run as root:
#   doas sh contrib/openbsd/install.sh
#
# What it does:
#   1. Creates the service user _http_smtp_rele
#   2. Installs the binary to /usr/local/bin/
#   3. Installs the config example to /etc/http-smtp-rele.toml (if absent)
#   4. Installs the rc.d script
#   5. Creates required directories
#   6. Prints next steps

set -e

BINARY="${1:-./target/release/http-smtp-rele}"
RC_SCRIPT="./contrib/openbsd/rc.d/http_smtp_rele"
CONFIG_EXAMPLE="./examples/http-smtp-rele.toml"

echo "==> http-smtp-rele installer for OpenBSD"

# 1. Service user
if ! id _http_smtp_rele >/dev/null 2>&1; then
    echo "==> Creating service user _http_smtp_rele"
    useradd -r -s /sbin/nologin -d /var/empty _http_smtp_rele
else
    echo "==> Service user _http_smtp_rele already exists"
fi

# 2. Binary
echo "==> Installing binary to /usr/local/bin/http-smtp-rele"
install -m 555 -o root -g bin "${BINARY}" /usr/local/bin/http-smtp-rele

# 3. Config
if [ ! -f /etc/http-smtp-rele.toml ]; then
    echo "==> Installing example config to /etc/http-smtp-rele.toml"
    install -m 640 -o root -g _http_smtp_rele "${CONFIG_EXAMPLE}" \
        /etc/http-smtp-rele.toml
    echo ""
    echo "    IMPORTANT: Edit /etc/http-smtp-rele.toml before starting:"
    echo "      [mail] default_from, allowed_recipient_domains"
    echo "      [[api_keys]] id, secret"
    echo ""
else
    echo "==> /etc/http-smtp-rele.toml already exists — not overwritten"
fi

# 4. rc.d script
echo "==> Installing rc.d script to /etc/rc.d/http_smtp_rele"
install -m 755 -o root -g bin "${RC_SCRIPT}" /etc/rc.d/http_smtp_rele

# 5. Directories
echo "==> Creating runtime directories"
install -d -o _http_smtp_rele -m 750 /var/db/http-smtp-rele   # SQLite store
install -d -o _http_smtp_rele -m 750 /var/log/http-smtp-rele  # optional logs

echo ""
echo "==> Done. Next steps:"
echo ""
echo "    1. Edit /etc/http-smtp-rele.toml"
echo "    2. rcctl enable http_smtp_rele"
echo "    3. rcctl start  http_smtp_rele"
echo "    4. rcctl check  http_smtp_rele"
echo ""
echo "    Config reload (no restart needed):"
echo "    rcctl reload http_smtp_rele"
