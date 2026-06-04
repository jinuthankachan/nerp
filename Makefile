# nERP — common developer & ops tasks.
# Requires Rust/cargo, Node + npm (e.g. via nvm) on PATH, and Docker.

COMPOSE := docker compose -f docker/compose.yaml
IMAGE   := nerp:latest

.DEFAULT_GOAL := help
.PHONY: help setup dev build test docker-build docker-up docker-logs docker-down clean

help: ## List available targets
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | \
		awk 'BEGIN{FS=":.*?## "}{printf "  \033[36m%-13s\033[0m %s\n", $$1, $$2}'

setup: ## Install dependencies (npm packages + fetch cargo crates)
	npm install
	cargo fetch

dev: ## Run locally: Postgres (Docker) + Vite asset watch + loco server
	$(COMPOSE) up -d postgres
	npm run watch & \
	WATCH=$$!; \
	trap 'kill $$WATCH 2>/dev/null' EXIT; \
	cargo loco start

build: ## Build frontend assets (Vite) and compile the app (cargo)
	npm run build
	cargo build

test: ## Run the Rust test suite
	cargo test

docker-build: ## Build the production single image
	docker build -f docker/Dockerfile -t $(IMAGE) .

docker-up: ## Start the full stack (Postgres + app) in the background
	$(COMPOSE) up -d --build

docker-logs: ## Follow logs from the running stack
	$(COMPOSE) logs -f

docker-down: ## Stop the stack (keeps the pgdata volume)
	$(COMPOSE) down

clean: ## Remove build artifacts (cargo target + Vite dist)
	cargo clean
	rm -rf assets/static/dist
