FROM rust:1.90-bookworm AS builder

RUN cargo install cargo-binstall --locked
RUN cargo binstall cargo-leptos --locked --no-confirm
RUN rustup target add wasm32-unknown-unknown

# Install Node.js (for npx tailwindcss) from NodeSource's deb
# repository. Previous incarnation piped `curl … | bash` — the script
# is vendor-maintained and has worked, but we now install the repo
# configuration with apt-get so the package list lives in the image
# instead of an opaque setup script.
RUN apt-get update \
    && apt-get install -y --no-install-recommends curl gnupg ca-certificates \
    && mkdir -p /etc/apt/keyrings \
    && curl -fsSL https://deb.nodesource.com/gpgkey/nodesource-repo.gpg.key \
        | gpg --dearmor -o /etc/apt/keyrings/nodesource.gpg \
    && echo "deb [signed-by=/etc/apt/keyrings/nodesource.gpg] https://deb.nodesource.com/node_22.x nodistro main" \
        > /etc/apt/sources.list.d/nodesource.list \
    && apt-get update \
    && apt-get install -y --no-install-recommends nodejs \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci

COPY . .
RUN npx tailwindcss -i style/input.css -o style/output.css --config tailwind.config.js --minify
RUN cargo leptos build --release

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 curl \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system --gid 1001 koentji \
    && useradd  --system --uid 1001 --gid koentji --home-dir /app --shell /usr/sbin/nologin koentji

WORKDIR /app
COPY --from=builder --chown=koentji:koentji /app/target/release/koentji .
COPY --from=builder --chown=koentji:koentji /app/target/site ./target/site

ENV LEPTOS_SITE_ADDR="0.0.0.0:3000"
ENV LEPTOS_SITE_ROOT="target/site"

EXPOSE 3000

# Drop to a non-root user so a compromise of the process doesn't hand
# the attacker root inside the container. `nologin` shell + system
# account keeps the surface small.
USER koentji

# HEALTHCHECK probes /healthz (liveness) — a DB blip must not restart
# the container, so we deliberately don't hit /readyz here.
# --start-period gives the bootstrap a grace window before the first
# probe counts toward the restart policy.
HEALTHCHECK --interval=30s --timeout=3s --start-period=20s --retries=3 \
    CMD curl --fail --silent --show-error http://localhost:3000/healthz || exit 1

CMD ["./koentji"]
