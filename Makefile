.PHONY: help build test deploy stop clean logs

# Colors for output
RED=\033[0;31m
GREEN=\033[0;32m
YELLOW=\033[1;33m
NC=\033[0m # No Color

help: ## Show this help message
	@echo "$(GREEN)Barter Trading System - Make Commands$(NC)"
	@echo ""
	@echo "Available commands:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(YELLOW)%-15s$(NC) %s\n", $$1, $$2}'

build: ## Build the Docker images
	@echo "$(GREEN)Building Docker images...$(NC)"
	docker-compose build

test: ## Run all tests
	@echo "$(GREEN)Running tests...$(NC)"
	cargo test --all-features --workspace

check: ## Run cargo check
	@echo "$(GREEN)Running cargo check...$(NC)"
	cargo check --all-features --workspace

lint: ## Run linting checks
	@echo "$(GREEN)Running lints...$(NC)"
	cargo fmt --all -- --check
	cargo clippy --all-features --workspace -- -D warnings

deploy: ## Deploy locally with docker compose
	@echo "$(GREEN)Checking port availability...$(NC)"
	@if lsof -Pi :18080 -sTCP:LISTEN -t >/dev/null ; then \
		echo "$(RED)Port 18080 is already in use!$(NC)"; \
		exit 1; \
	fi
	@if lsof -Pi :16379 -sTCP:LISTEN -t >/dev/null ; then \
		echo "$(RED)Port 16379 is already in use!$(NC)"; \
		exit 1; \
	fi
	@if lsof -Pi :15432 -sTCP:LISTEN -t >/dev/null ; then \
		echo "$(RED)Port 15432 is already in use!$(NC)"; \
		exit 1; \
	fi
	@if lsof -Pi :13000 -sTCP:LISTEN -t >/dev/null ; then \
		echo "$(RED)Port 13000 is already in use!$(NC)"; \
		exit 1; \
	fi
	@echo "$(GREEN)All ports are available. Starting deployment...$(NC)"
	docker compose up -d
	@echo "$(GREEN)Deployment complete! Services are running on:$(NC)"
	@echo "  - Barter Engine: http://localhost:18080"
	@echo "  - Redis: localhost:16379"
	@echo "  - PostgreSQL: localhost:15432"
	@echo "  - Grafana: http://localhost:13000"

stop: ## Stop all running containers
	@echo "$(YELLOW)Stopping all containers...$(NC)"
	docker compose down

clean: ## Clean up containers and volumes
	@echo "$(RED)Cleaning up containers and volumes...$(NC)"
	docker compose down -v
	rm -rf data/ logs/

logs: ## Show logs from all containers
	docker compose logs -f

logs-barter: ## Show logs from barter container only
	docker compose logs -f barter

status: ## Show status of all containers
	@echo "$(GREEN)Container Status:$(NC)"
	docker compose ps

restart: ## Restart all services
	@echo "$(YELLOW)Restarting services...$(NC)"
	docker compose restart

shell: ## Enter barter container shell
	docker compose exec barter /bin/bash

db-shell: ## Enter PostgreSQL shell
	docker compose exec postgres psql -U barter -d barter_trading

redis-cli: ## Enter Redis CLI
	docker compose exec redis redis-cli

backup: ## Backup database
	@echo "$(GREEN)Backing up database...$(NC)"
	@mkdir -p backups
	docker compose exec postgres pg_dump -U barter barter_trading | gzip > backups/backup_$$(date +%Y%m%d_%H%M%S).sql.gz
	@echo "$(GREEN)Backup completed!$(NC)"