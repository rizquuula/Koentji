FROM rust:1.90 AS builder

RUN cargo install cargo-binstall --locked
RUN cargo binstall cargo-leptos --locked --no-confirm
RUN rustup target add wasm32-unknown-unknown

# Install Node.js (for npx tailwindcss)
RUN curl -fsSL https://deb.nodesource.com/setup_22.x | bash - \
    && apt-get install -y nodejs

WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci

COPY . .
RUN npx tailwindcss -i style/input.css -o style/output.css --config tailwind.config.js --minify
RUN cargo leptos build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/koentji .
COPY --from=builder /app/target/site ./target/site

ENV LEPTOS_SITE_ADDR="0.0.0.0:3000"
ENV LEPTOS_SITE_ROOT="target/site"

EXPOSE 3000

CMD ["./koentji"]
