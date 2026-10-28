SHELL := /bin/bash

.PHONY: help version hooks backend-build backend-test backend-test-db backend-fmt backend-lint \
        frontend-install frontend-build frontend-test frontend-api \
        integration-test upgrade-test boundaries secrets validate test build up down \
        backup restore upgrade destroy clean

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'

version: ## Print the authoritative product version (MAJOR.MINOR.yyyymmdd)
	./scripts/version.sh

backend-build: ## Build the Rust backend workspace
	cd backend && cargo build --workspace

backend-test: ## Run backend tests (no database required)
	cd backend && cargo test --workspace

backend-test-db: ## Run backend repository tests against a throwaway Postgres
	./scripts/test-backend-db.sh

backend-fmt: ## Check backend formatting
	cd backend && cargo fmt --all -- --check

backend-lint: ## Lint the backend with clippy (warnings are errors)
	cd backend && cargo clippy --workspace --all-targets -- -D warnings

frontend-install: ## Install frontend dependencies
	cd frontend && npm ci

frontend-build: ## Type-check and build the frontend
	cd frontend && npm run build

frontend-test: ## Run frontend tests
	cd frontend && npm test

frontend-api: ## Regenerate the frontend typed client from the OpenAPI contract
	cd frontend && npm run generate:api

integration-test: ## Run the cross-boundary integration test (live backend + frontend)
	./scripts/integration-test.sh

upgrade-test: ## Verify schema upgrade, version, encrypted backup, and restore
	./scripts/test-upgrade.sh

boundaries: ## Verify there are no cross-project imports
	./scripts/check-boundaries.sh

hooks: ## Install the repo git hooks (gitleaks pre-commit)
	git config core.hooksPath .githooks

secrets: ## Scan tracked files and full history for credentials
	./scripts/check-secrets.sh
	./scripts/run-gitleaks.sh git --redact --no-banner --config .gitleaks.toml

validate: ## Validate the OpenSpec change strictly
	openspec validate networth-tracker-v1 --strict

test: secrets backend-test frontend-test boundaries ## Run all tests and checks

build: backend-build frontend-build ## Build both projects

up: ## Start the full local stack (db, backend, frontend)
	docker compose up --build

down: ## Stop the local stack (keeps the database volume)
	docker compose down

backup: ## Back up the database to ./backups
	./scripts/backup.sh

restore: ## Restore a backup: make restore FILE=backups/networth-....sql.gz.gpg
	./scripts/restore.sh "$(FILE)"

upgrade: ## Back up, rebuild, and restart without losing data
	./scripts/upgrade.sh

destroy: ## Permanently delete the database volume (confirmation required)
	./scripts/destroy.sh

clean: ## Remove build artifacts
	cd backend && cargo clean
	rm -rf frontend/dist
