SHELL := /bin/bash
COMPOSE ?= docker compose

.PHONY: help version hooks backend-build backend-test backend-test-db backend-fmt backend-lint \
        frontend-install frontend-build frontend-test frontend-api \
        integration-test upgrade-test upgrade-matrix entrypoint-test offline-test e2e-test verify-guards boundaries secrets validate test build up down \
        backup backup-scheduled backup-timer backup-schedule-test backup-container-test documents-cleanup documents-cleanup-apply documents-cleanup-test restore restore-drill upgrade destroy clean

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

upgrade-matrix: ## Upgrade fixture databases from recent release tags
	./scripts/test-upgrade-matrix.sh

entrypoint-test: ## Verify pre-migration backup detection and fail-closed startup
	./scripts/test-entrypoint.sh

offline-test: ## Verify core features with no providers configured
	./scripts/offline-test.sh

e2e-test: ## Run the full local release/e2e matrix
	$(MAKE) test
	$(MAKE) backend-test-db
	$(MAKE) integration-test
	$(MAKE) upgrade-test
	$(MAKE) entrypoint-test
	$(MAKE) backup-schedule-test
	$(MAKE) backup-container-test
	$(MAKE) documents-cleanup-test
	$(MAKE) offline-test

verify-guards: ## Prove contract and boundary checks fail on deliberate violations
	./scripts/verify-guards.sh

boundaries: ## Verify there are no cross-project imports
	./scripts/check-boundaries.sh

hooks: ## Install the repo git hooks (gitleaks pre-commit)
	git config core.hooksPath .githooks

secrets: ## Scan tracked files and full history for credentials
	./scripts/check-secrets.sh
	./scripts/run-gitleaks.sh git --redact --no-banner --config .gitleaks.toml

validate: ## Validate all OpenSpec changes and specs strictly
	openspec validate --all --strict

test: secrets backend-test frontend-test boundaries backup-container-test ## Run all tests and checks

build: backend-build frontend-build ## Build both projects

up: ## Start the full local stack (db, backend, frontend)
	$(COMPOSE) up --build

down: ## Stop the local stack (keeps the database volume)
	$(COMPOSE) down

backup: ## Back up the database to ./backups
	./scripts/backup.sh

backup-scheduled: ## Run the cron/systemd backup wrapper once
	./scripts/scheduled-backup.sh

backup-timer: ## Print the systemd user timer (use --install to write it)
	./scripts/install-backup-timer.sh

backup-schedule-test: ## Test scheduled backup locking, logging, and fail-closed behavior
	./scripts/test-backup-schedule.sh

backup-container-test: ## Test the in-container backup script with stubbed pg_dump
	./scripts/test-container-backup.sh

documents-cleanup: ## Dry-run orphaned/temporary document file cleanup
	./scripts/cleanup-documents.sh

documents-cleanup-apply: ## Remove confirmed orphaned/temporary document files
	./scripts/cleanup-documents.sh --apply

documents-cleanup-test: ## Test document cleanup candidates and confirmation
	./scripts/test-document-cleanup.sh

restore: ## Restore a backup: make restore FILE=backups/networth-....sql.gz.gpg
	./scripts/restore.sh "$(FILE)"

restore-drill: ## Restore newest backup into a throwaway DB and verify rows
	./scripts/restore-drill.sh $(FILE)

upgrade: ## Back up, rebuild, and restart without losing data
	./scripts/upgrade.sh

destroy: ## Permanently delete the database volume (confirmation required)
	./scripts/destroy.sh

clean: ## Remove build artifacts
	cd backend && cargo clean
	rm -rf frontend/dist
