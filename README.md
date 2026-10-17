# NetworthTracker

A local-first, offline-capable personal and family net worth tracker. It keeps
your data on your own machine, works without a network connection for manual
entry, and reports net worth in *useful* forms: total, liquid, and accessible.

This repository is a **monorepo** with two decoupled projects:

```
backend/     Rust API (Axum + SQLx + Postgres)      -> its own Dockerfile
frontend/    TypeScript web app (Vite + React)      -> its own Dockerfile
contracts/   Shared OpenAPI contract (the only integration surface)
docs/        Shared documentation
openspec/    Spec-driven change proposals and specs
scripts/     Root tooling (boundary checks, etc.)
docker-compose.yml / Makefile / .github  -> root orchestration
```

## Decoupling rule

The frontend is only a consumer of the backend HTTP API. The two projects share
no code:

- `backend/` must not reference `frontend/`, and `frontend/` must not reference
  `backend/`.
- Integration happens exclusively over the HTTP API described in
  `contracts/openapi.yaml`.
- Shared, non-runtime assets (docs, container orchestration, CI, the contract)
  live at the repository root.

`make boundaries` enforces this in CI.

## Quickstart

Everything, in containers:

```bash
make up        # db + backend + frontend, then open http://localhost:3000
```

Or run the pieces separately for development — see `docs/development.md`.

| Service  | URL                     |
|----------|-------------------------|
| Frontend | http://localhost:3000   |
| Backend  | http://localhost:8080   |
| Postgres | internal (`db:5432`)    |

Host ports are overridable, e.g. `FRONTEND_PORT=8081 docker compose up`.

## Development

```bash
make help             # list targets
make backend-test     # cargo test
make frontend-test    # vitest
make boundaries       # no cross-project imports
make validate         # openspec validate --strict
make test             # all of the above
```

## Design

The v1 design lives in OpenSpec:

- `openspec/changes/networth-tracker-v1/` — proposal, specs, design, tasks

Key decisions: PostgreSQL with exact `NUMERIC` money (`bigdecimal`), a
hexagonal backend, liquidity and accessibility as separate axes, immutable net
worth snapshots, a spreadsheet-like bulk-entry grid, and user-funded
bring-your-own-key data add-ons.
