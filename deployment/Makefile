.DEFAULT_GOAL := help

DC := docker compose

.PHONY: help
help: ## Show this help
	@awk 'BEGIN {FS = ":.*##"; printf "Usage: make \033[36m<target>\033[0m\n\nTargets:\n"} \
	  /^[a-zA-Z_-]+:.*?##/ { printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2 }' $(MAKEFILE_LIST)

.PHONY: up
up: ## Start all services in detached mode
	$(DC) up -d

.PHONY: down
down: ## Stop and remove containers
	$(DC) down

.PHONY: restart
restart: ## Restart all services
	$(DC) restart

.PHONY: build
build: ## Build (or rebuild) images
	$(DC) build

.PHONY: logs
logs: ## Follow logs (all services); pass s=<service> to filter
	$(DC) logs -f $(s)

.PHONY: ps
ps: ## Show running containers
	$(DC) ps

.PHONY: shell
shell: ## Open a shell in the app container
	$(DC) exec app sh

.PHONY: db-shell
db-shell: ## Open a psql shell in the db container
	$(DC) exec db psql -U $${POSTGRES_USER:-koentji} $${POSTGRES_DB:-koentjilab}

.PHONY: pull
pull: ## Pull latest koentji app image
	docker pull ghcr.io/rizquuula/koentji:latest

.PHONY: dump-db
dump-db: ## Dump all DB data to koentji-db-<datetime>.sql
	docker exec koentji-db pg_dump \
	  -U $${POSTGRES_USER:-koentji} \
	  $${POSTGRES_DB:-koentjilab} \
	  > koentji-db-$$(date -u +%Y-%m-%dT%H:%M:%SZ).sql

.PHONY: clean
clean: ## Stop containers and remove volumes (destructive)
	$(DC) down -v
