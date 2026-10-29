# AGENTS.md

Conventions for AI coding agents working in this repository. Follow these unless
the user explicitly overrides them.

## What this project is

NetworthTracker — a local-first, offline-capable personal and family net worth
tracker. Rust backend + TypeScript frontend + PostgreSQL, run locally in
containers. Design lives in `openspec/changes/networth-tracker-v1/`.

## Repository layout

```
backend/     Rust API (Axum + SQLx)      -> own Dockerfile
frontend/    TypeScript app (Vite+React) -> own Dockerfile
contracts/   openapi.yaml (shared; the only integration surface)
docs/        shared docs
scripts/     root tooling
openspec/    spec-driven change proposals and specs
```

## Hard rules

1. **Decoupled projects.** `backend/` and `frontend/` share no code. The backend
   must not reference `frontend/`, and the frontend must not reference
   `backend/`. They integrate only over the HTTP API in `contracts/openapi.yaml`.
   Shared, non-runtime assets live at the repository root. `make boundaries`
   enforces this; CI fails on violations.
2. **Grow together.** A change that adds API capability also adds its frontend
   consumer and a cross-boundary test in the same change (`make integration-test`).
3. **Exact money, never floats.** Monetary and crypto amounts use
   arbitrary-precision decimals (`bigdecimal` in Rust), are currency-tagged, and
   cross JSON as strings. Never use `f32`/`f64` or SQL `REAL` for money.
4. **Offline-first.** Core features (manual entry, net worth, history) must work
   with no network. External providers are optional, user-funded add-ons behind
   read-only adapter traits; nothing is bundled and there is no central service.
5. **One contract.** Update `contracts/openapi.yaml` when endpoints change and
   regenerate the frontend client with `make frontend-api`; `npm run check:api`
   must pass.
6. **Verify before claiming done.** Run the relevant checks (`make test`,
   `make integration-test`, `make validate`) and report what actually passed.

## Secrets and credentials

- **Never commit real passwords, API keys, tokens, private keys, or `.env`
  files.** Local secrets live in `.env`, which is gitignored; `.env.example`
  documents the required keys with placeholders.
- Use obvious placeholders in docs (`change-me`, `your-api-key`) and clearly fake
  values in tests (`test-only-not-a-secret`). Never paste a credential from a
  real account into the repository.
- Run `make secrets` before every commit. It scans tracked files for private
  keys, cloud/API tokens, and database URLs that contain a password.
- The release pipeline also runs **gitleaks** over the full git history. Its
  allowlist must be narrow and justified: fake test fixtures and historical dev
  defaults only, never a real credential. See task 14.1.
- The pre-commit hook (`.githooks/pre-commit`, enabled with `make hooks`) runs
  the fast scan and `gitleaks git --staged` before every commit. `make secrets`
  also scans the full history. The allowlist lives in `.gitleaks.toml` and is
  documented in `docs/security.md`.
- If a credential is ever committed, treat it as compromised: rotate it first,
  then remove it from history. Deleting the file is not enough.
- Provider credentials (market data, aggregation, chains) are supplied by the
  individual user and stored locally; the product bundles none and runs no
  central service.
- The database password comes from `.env`; there is no committed default.

## Versioning

The product uses **date-based semantic versioning** in the format
**`MAJOR.MINOR.yyyymmdd`**:

- `MAJOR` — incremented for incompatible changes; `MINOR` resets to `0`.
- `MINOR` — incremented for backwards-compatible features.
- `yyyymmdd` — the **UTC release date**, replacing the usual numeric patch.

Example: a release tagged on 17 October 2026 is **`0.1.20261017`**.

Single source of truth: `backend/Cargo.toml` → `[workspace.package].version`.
Mirror it into `frontend/package.json` and `contracts/openapi.yaml`
(`info.version`). The running API reports it via `GET /api/v1/ping`, and release
container images are tagged `v<version>`.

- Read the authoritative value with `make version`.
- Do not invent ad-hoc versions; `0.x.yyyymmdd` is the only allowed shape.

### Changelog

- `CHANGELOG.md` records every tagged release using the Keep a Changelog format.
- Add a dated section named after the version in the same commit that bumps the
  version, before creating the tag.
- Use `Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, and `Security`
  headings as needed; keep `[Unreleased]` at the top.
- The version, changelog section, and tag must match.

## Common commands

```bash
make help              # list targets
make test              # backend + frontend tests + secret + boundary checks
make upgrade-test      # schema upgrade + version + encrypted backup/restore
make secrets           # scan tracked files for credentials
make integration-test  # boot the backend, drive it through the frontend client
make validate          # openspec validate --strict
make up                # full local stack (db, backend, frontend)
```

## Working with OpenSpec

Changes follow `proposal → specs → design → tasks`. During the propose workflow,
write planning artifacts only — do not implement. Implementation happens in the
apply workflow: work through `tasks.md`, tick checkboxes as you verify each task,
and keep specs/design in sync with reality.
