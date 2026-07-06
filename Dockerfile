FROM rust:1.91-bookworm AS builder

# Fetch cargo-binstall's own prebuilt binary instead of compiling it.
# `cargo install cargo-binstall` builds the latest release from source,
# which couples us to whatever rustc *that* release demands — v1.20.1
# pulls vergen 10 and needs rustc 1.95, so it fails to compile on this
# 1.91 image. The prebuilt musl binary has no such coupling (and is far
# faster). Explicit release tarball over the upstream `curl | bash`
# installer, matching the NodeSource setup below. amd64-only, in step
# with the single-arch image the CI builds.
RUN curl -fsSL https://github.com/cargo-bins/cargo-binstall/releases/latest/download/cargo-binstall-x86_64-unknown-linux-musl.tgz \
        | tar -xzf - -C "${CARGO_HOME:-/usr/local/cargo}/bin"
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
    && apt-get install -y --no-install-recommends ca-certificates libssl3 curl libfontconfig1 \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system --gid 1001 koentji \
    && useradd  --system --uid 1001 --gid koentji --home-dir /app --shell /usr/sbin/nologin koentji

WORKDIR /app
COPY --from=builder --chown=koentji:koentji /app/target/release/koentji .
COPY --from=builder --chown=koentji:koentji /app/target/site ./target/site
# hash-files=true renames the bundle to koentji.<hash>.{js,wasm,css}. At
# runtime HydrationScripts/HashedStylesheet resolve those names by reading
# hash.txt from the binary's own directory (std::env::current_exe()), so it
# must sit next to ./koentji. cargo-leptos writes it beside the server binary.
COPY --from=builder --chown=koentji:koentji /app/target/release/hash.txt ./hash.txt

ENV LEPTOS_SITE_ADDR="0.0.0.0:3000"
ENV LEPTOS_SITE_ROOT="target/site"
# hash_files is read from the runtime env (it is NOT baked into the binary the
# way output_name is), so without this every hashed asset 404s in production.
ENV LEPTOS_HASH_FILES="true"

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
