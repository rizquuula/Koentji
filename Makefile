.PHONY: dev build fmt clippy test clean migrate db-create db-reset seed docker-up docker-down tailwind

# Development
dev: tailwind
	cargo leptos watch

build: tailwind
	cargo leptos build --release

# TailwindCSS
tailwind:
	npx tailwindcss -i style/input.css -o style/output.css --minify

tailwind-watch:
	npx tailwindcss -i style/input.css -o style/output.css --watch

# Database
db-create:
	createdb koentjilab || true

db-reset:
	dropdb koentjilab || true
	createdb koentjilab
	$(MAKE) migrate

migrate:
	psql $(DATABASE_URL) -f migrations/001_create_auth_keys.sql

seed:
	psql $(DATABASE_URL) -f migrations/seed.sql

# Code quality
fmt:
	cargo fmt

clippy:
	cargo clippy --all-features -- -D warnings

test:
	cargo test

clean:
	cargo clean

# Docker
docker-up:
	docker compose up -d

docker-down:
	docker compose down

docker-build:
	docker compose build
