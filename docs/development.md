# Development

The backend and frontend are decoupled projects. You can run either on its own,
or both together with Compose.

## Prerequisites

- Rust (stable) and Cargo
- Node 22+ and npm
- Docker or Podman with Compose (for the full stack)

## Backend

```bash
cd backend
cargo run -p networth-api        # listens on 0.0.0.0:8080
cargo test --workspace
```

Configuration (environment variables):

| Variable               | Default                                                    |
|------------------------|------------------------------------------------------------|
| `BACKEND_ADDR`         | `0.0.0.0:8080`                                             |
| `CORS_ALLOWED_ORIGINS` | `http://localhost:5173,http://localhost:3000`              |
| `DATABASE_URL`         | (not used yet; wired when persistence lands)               |

Check it:

```bash
curl -s localhost:8080/health
curl -s localhost:8080/api/v1/ping
```

## Frontend

```bash
cd frontend
npm ci
npm run dev                      # http://localhost:5173
```

The API base URL is read from `VITE_API_BASE_URL` (see `.env.example`). Copy it
to `.env.local` to override; the default is `http://localhost:8080`.

Tests and the typed client:

```bash
npm test
npm run generate:api             # regenerate src/api/schema.d.ts from the contract
npm run check:api                # fail if the generated client is stale
```

## Full stack

```bash
make up                          # docker compose up --build
open http://localhost:3000
```

The frontend container is served by nginx and calls the backend at
`http://localhost:8080` (baked in at build time via the `VITE_API_BASE_URL`
build arg), because the browser — not the container network — resolves that URL.

Host ports default to `3000` (frontend) and `8080` (backend) and can be
overridden; the CORS allowlist and the baked-in API URL follow the variables:

```bash
FRONTEND_PORT=8081 docker compose up --build
```

## The contract

`contracts/openapi.yaml` is the single integration surface. The backend's
responses are validated against it and the frontend's typed client is generated
from it, so the two projects cannot drift silently:

```bash
make frontend-api                # regenerate types from the contract
./scripts/check-boundaries.sh    # no cross-project imports
```
