.PHONY: help dev run build version fmt fmt-check clippy test test-rust test-e2e test-e2e-install check clean migrate db-create db-reset docker-up docker-up-local docker-down docker-up-db docker-logs docker-pull docker-update-app tailwind refactor-status refactor-next hash-admin-password

-include .env
export

.DEFAULT_GOAL := help

## Help
help: ## Show this help message
	@echo "Usage: make [target]"
	@awk ' \
		/^## / { \
			header = substr($$0, 4); \
			printf "\n\033[1;33m%s\033[0m\n", header; \
			next; \
		} \
		/^[a-zA-Z0-9_-]+:.*?## / { \
			split($$0, parts, ":.*?## "); \
			printf "  \033[36m%-22s\033[0m %s\n", parts[1], parts[2]; \
		} \
	' $(firstword $(MAKEFILE_LIST))

## Development
dev: ## Run dev server with cargo leptos watch
	cargo leptos watch

run: ## Run server without watching for changes
	cargo leptos serve

build: ## Build release binary
	cargo leptos build --release

version: ## Print the current package version (from Cargo.toml)
	@grep -m1 '^version = ' Cargo.toml | cut -d '"' -f2

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

migrate: ## Run pending migrations
	cargo run --features ssr -- run-migrations

## Code quality
fmt: ## Format code
	cargo fmt

fmt-check: ## Check formatting without rewriting (CI-safe)
	cargo fmt --check

clippy: ## Run clippy lints (all features, deny warnings)
	cargo clippy --all-features --tests -- -D warnings

## Tests
test-rust: ## Run Rust tests with ssr feature enabled
	# Integration tests share one Postgres DB and coordinate via
	# `reset()` — running them in parallel lets a TRUNCATE wipe another
	# test's rows mid-flight, which in turn flakes the concurrency
	# tests. `--test-threads=1` serialises within each test binary;
	# cargo still runs separate binaries sequentially.
	cargo test --features ssr -- --test-threads=1

test-e2e-install: ## Install Playwright browsers and dependencies
	cd end2end && npm install && npx playwright install --with-deps chromium

test-e2e: ## Run Playwright end-to-end test suite
	cd end2end && npx playwright test

test: test-rust test-e2e ## Run all tests (Rust + Playwright)

check: fmt-check clippy test-rust ## fmt --check + clippy -D warnings + cargo test (safety gate)

clean: ## Clean build artifacts
	cargo clean

## Docker
docker-up: ## Start all containers
	docker compose up -d

docker-up-local: ## Build local image (koentji:local) and start all containers
	APP_IMAGE=koentji:local docker compose up -d --build

docker-up-db: ## Start only the database container (port 5432 exposed)
	docker compose up -d db

docker-down: ## Stop all containers
	docker compose down

docker-build: ## Build Docker images
	docker compose build

docker-logs: ## Tail logs for all containers (or pass s=service to filter)
	docker compose logs -f $(s)

docker-pull: ## Pull latest Docker images
	docker compose pull

docker-update-app: ## Pull the latest app image and recreate only the app container (override tag with APP_IMAGE=...)
	docker compose pull app
	docker compose up -d app

## Admin access
hash-admin-password: ## Print an argon2id PHC hash for ADMIN_PASSWORD_HASH (pass PASSWORD=...)
	@if [ -z "$(PASSWORD)" ]; then \
		echo "usage: make hash-admin-password PASSWORD=yourpassword" >&2; \
		exit 1; \
	fi
	@cargo run --quiet --features ssr --bin hash-admin-password -- "$(PASSWORD)"

## Refactor progress (staged DDD remediation)
refactor-status: ## Show staged refactor progress
	@sed -n '/## Checklist/,/## Log/p' .claude-refactor/PROGRESS.md

refactor-next: ## Print the next unchecked refactor commit
	@grep -m1 '^- \[ \]' .claude-refactor/PROGRESS.md || echo "All done."
