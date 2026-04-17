.PHONY: help dev run build fmt fmt-check clippy test check clean migrate db-create db-reset docker-up docker-down docker-up-db docker-logs docker-pull tailwind e2e e2e-install refactor-status refactor-next hash-admin-password

-include .env
export

.DEFAULT_GOAL := help

## Help
help: ## Show this help message
	@echo "Usage: make [target]"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(firstword $(MAKEFILE_LIST)) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'

## Development
dev: ## Run dev server with cargo leptos watch
	cargo leptos watch

run: ## Run server without watching for changes
	cargo leptos serve

build: ## Build release binary
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

migrate: ## Run pending migrations
	cargo run --features ssr -- run-migrations

## Code quality
fmt: ## Format code
	cargo fmt

fmt-check: ## Check formatting without rewriting (CI-safe)
	cargo fmt --check

clippy: ## Run clippy lints (all features, deny warnings)
	cargo clippy --all-features --tests -- -D warnings

test: ## Run Rust tests with ssr feature enabled
	# Integration tests share one Postgres DB and coordinate via
	# `reset()` — running them in parallel lets a TRUNCATE wipe another
	# test's rows mid-flight, which in turn flakes the concurrency
	# tests. `--test-threads=1` serialises within each test binary;
	# cargo still runs separate binaries sequentially.
	cargo test --features ssr -- --test-threads=1

check: fmt-check clippy test ## fmt --check + clippy -D warnings + cargo test (safety gate)

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

docker-logs: ## Tail logs for all containers (or pass s=service to filter)
	docker compose logs -f $(s)

docker-pull: ## Pull latest Docker images
	docker compose pull

## End-to-end tests
e2e-install: ## Install Playwright browsers and dependencies
	cd end2end && npm install && npx playwright install --with-deps chromium

e2e: ## Run Playwright end-to-end test suite
	cd end2end && npx playwright test

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
