#!/bin/sh
# Shruti server entrypoint — optional TLS via Caddy.
#
# Without TLS_ENABLED=true: runs shruti directly as non-root (plain HTTP on 8050).
# With TLS_ENABLED=true: starts supervisord managing both shruti (non-root) + Caddy (root).
set -e

export TLS_ENABLED="${TLS_ENABLED:-false}"
export TLS_CERT_PATH="${TLS_CERT_PATH:-}"
export TLS_KEY_PATH="${TLS_KEY_PATH:-}"
export TLS_DOMAIN="${TLS_DOMAIN:-localhost}"
export TLS_PORT="${TLS_PORT:-443}"

# Ensure /tmp is writable for supervisord pidfile and caddy override
chmod 1777 /tmp 2>/dev/null || true

if [ "$TLS_ENABLED" = "true" ]; then
    echo "[shruti] TLS enabled — configuring Caddy reverse proxy..."

    if [ -n "$TLS_CERT_PATH" ] && [ -n "$TLS_KEY_PATH" ]; then
        echo "[shruti]   Mode A: Using provided certificates"
        export TLS_CERT_DIRECTIVE="${TLS_CERT_PATH} ${TLS_KEY_PATH}"
    else
        echo "[shruti]   Mode B: Auto HTTPS (ACME) for domain ${TLS_DOMAIN}"
        export TLS_CERT_DIRECTIVE=""
    fi

    envsubst < /etc/caddy/Caddyfile.template > /etc/caddy/Caddyfile

    # Enable Caddy in supervisord
    cat > /tmp/supervisord-caddy.conf <<EOF
[program:caddy]
autostart=true
EOF

    # supervisord manages both shruti (as user shruti) and caddy (as root)
    exec supervisord -c /etc/supervisord.conf
else
    echo "[shruti] TLS disabled — serving plain HTTP"

    # Run shruti directly as non-root — no supervisord overhead
    exec su -s /bin/sh shruti -c 'exec "$0" "$@"' -- "$@"
fi
