.PHONY: help dev run build fmt clippy test clean migrate db-create db-reset seed docker-up docker-down docker-up-db tailwind

.DEFAULT_GOAL := help

## Help
help: ## Show this help message
	@echo "Usage: make [target]"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'

## Development
dev: tailwind ## Run dev server with cargo leptos watch
	cargo leptos watch

run: tailwind ## Run server without watching for changes
	cargo leptos serve

build: tailwind ## Build release binary
	cargo leptos build --release

## TailwindCSS
tailwind: ## Build TailwindCSS (minified)
	npx tailwindcss -i style/input.css -o style/output.css --minify

tailwind-watch: ## Watch and rebuild TailwindCSS
	npx tailwindcss -i style/input.css -o style/output.css --watch

## Database
db-create: ## Create the database
	createdb koentjilab || true

db-reset: ## Drop and recreate the database
	dropdb koentjilab || true
	createdb koentjilab
	$(MAKE) migrate

migrate: ## Run database migrations
	psql $(DATABASE_URL) -f migrations/001_create_auth_keys.sql

seed: ## Seed the database
	psql $(DATABASE_URL) -f migrations/seed.sql

## Code quality
fmt: ## Format code
	cargo fmt

clippy: ## Run clippy lints
	cargo clippy --all-features -- -D warnings

test: ## Run tests
	cargo test

clean: ## Clean build artifacts
	cargo clean

## Docker
docker-up: ## Start all containers
	docker compose up -d

docker-up-db: ## Start only the database container (port 5432 exposed)
	docker compose up -d db

docker-down: ## Stop all containers
	docker compose down

docker-build: ## Build Docker images
	docker compose build
