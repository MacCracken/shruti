# Shruti DAW — Headless Server Image
# Runs `shruti serve` on port 8050 for SecureYeoman / AGNOS integration.
# No GUI dependencies at runtime — shruti-ui is linked but unused in serve mode.
#
# TLS modes (optional, via environment variables):
#   TLS_ENABLED=false (default) — Shruti serves plain HTTP on port 8050
#   TLS_ENABLED=true + TLS_CERT_PATH + TLS_KEY_PATH — Caddy terminates TLS with provided certs
#   TLS_ENABLED=true + TLS_DOMAIN — Caddy auto-obtains certs via ACME
#
# Build:  docker build -t shruti-server .
# Run:    docker run -p 8050:8050 shruti-server
# TLS:    docker run -p 443:443 -e TLS_ENABLED=true -e TLS_DOMAIN=shruti.example.com shruti-server

# ── Stage 1: Builder ────────────────────────────────────────────────
FROM rust:1.86-bookworm AS builder

# System packages needed at build time:
#   - ALSA (cpal audio backend)
#   - X11/Wayland/GPU libs (eframe/wgpu — linked but unused in serve mode)
#   - pkg-config for native dependency discovery
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libasound2-dev \
    libx11-dev \
    libxcursor-dev \
    libxrandr-dev \
    libxi-dev \
    libxkbcommon-dev \
    libwayland-dev \
    libgtk-3-dev \
    libvulkan-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy dependency manifests first for layer caching
COPY Cargo.toml Cargo.lock ./
COPY vendor/ vendor/
COPY crates/shruti-engine/Cargo.toml crates/shruti-engine/Cargo.toml
COPY crates/shruti-dsp/Cargo.toml crates/shruti-dsp/Cargo.toml
COPY crates/shruti-plugin/Cargo.toml crates/shruti-plugin/Cargo.toml
COPY crates/shruti-ui/Cargo.toml crates/shruti-ui/Cargo.toml
COPY crates/shruti-session/Cargo.toml crates/shruti-session/Cargo.toml
COPY crates/shruti-ai/Cargo.toml crates/shruti-ai/Cargo.toml
COPY crates/shruti-instruments/Cargo.toml crates/shruti-instruments/Cargo.toml

# Create stub lib.rs files so cargo can resolve the workspace
RUN for d in crates/*/; do mkdir -p "$d/src" && echo "" > "$d/src/lib.rs"; done \
    && mkdir -p src && echo "fn main() {}" > src/main.rs \
    && mkdir -p src/bin && echo "fn main() {}" > src/bin/play.rs

# Pre-fetch and compile dependencies (cached unless Cargo.toml/lock changes)
RUN cargo build --release --bin shruti 2>&1 || true

# Copy actual source code
COPY crates/ crates/
COPY src/ src/

# Touch source files to invalidate the stub build
RUN find crates/ src/ -name "*.rs" -exec touch {} + \
    && cargo build --release --bin shruti

# ── Stage 2: Runtime ───────────────────────────────────────────────
FROM debian:bookworm-slim

# Runtime deps: ALSA, Caddy (TLS termination), supervisord (process management)
RUN apt-get update && apt-get install -y --no-install-recommends \
    libasound2 \
    ca-certificates \
    tini \
    curl \
    supervisor \
    debian-keyring \
    debian-archive-keyring \
    apt-transport-https \
    gnupg \
    && curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg \
    && curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' | tee /etc/apt/sources.list.d/caddy-stable.list \
    && apt-get update && apt-get install -y --no-install-recommends caddy \
    && apt-get install -y --no-install-recommends gettext-base \
    && apt-get purge -y gnupg debian-keyring apt-transport-https \
    && apt-get autoremove -y \
    && rm -rf /var/lib/apt/lists/*

# Non-root user
RUN groupadd -r shruti && useradd -r -g shruti -s /sbin/nologin -m shruti

COPY --from=builder /build/target/release/shruti /usr/local/bin/shruti
RUN chmod +x /usr/local/bin/shruti

# ── Caddy TLS config ──────────────────────────────────────────────
COPY docker/Caddyfile.template /etc/caddy/Caddyfile.template

# ── Supervisord config ────────────────────────────────────────────
COPY docker/supervisord.conf /etc/supervisord.conf

# ── Entrypoint ────────────────────────────────────────────────────
COPY docker/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

# Caddy data/config dirs
RUN mkdir -p /var/lib/caddy /etc/caddy \
    && chown shruti:shruti /var/lib/caddy

EXPOSE 8050 443

HEALTHCHECK --interval=15s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -sf http://127.0.0.1:8050/health || exit 1

ENTRYPOINT ["tini", "--", "/entrypoint.sh"]
CMD ["shruti", "serve", "--port", "8050"]
