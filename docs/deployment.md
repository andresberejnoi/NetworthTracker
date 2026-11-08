# Deployment

NetworthTracker is a local, self-hosted app: PostgreSQL, a Rust API, and a
static frontend. It runs with Docker Compose or Podman Compose; there is no
cloud service or bundled provider credential.

## Requirements

- Docker 24+ with Compose v2, **or** Podman 4.7+ with a Compose provider
  (`podman compose`, `podman-compose`, or the Docker compatibility shim).
- 2 GB RAM and a few GB of disk for the database, images, and backups.
- Ports for the frontend and backend (defaults `3000` and `8080`).
- A browser on the same machine or LAN.

The repository includes `docker-compose.yml`, `backend/Dockerfile`, and
`frontend/Dockerfile`.

## Configuration

```bash
cp .env.example .env
```

At minimum, change:

| Variable | Purpose |
|---|---|
| `POSTGRES_PASSWORD` | Local database password |
| `BACKUP_PASSPHRASE` | Required for encrypted backups; store it separately |
| `FRONTEND_PORT` | Frontend host port, default `3000` |
| `BACKEND_PORT` | API host port, default `8080` |
| `AUTH_DISABLED` | `false` for login, `true` for single-user offline mode |
| `IMAGE_TAG` | Image version, default `latest`; pin a release such as `v0.1.20261101` |
| `COMPOSE_CMD` | Compose command used by scripts, e.g. `podman compose` |
| `UPGRADE_PULL` | `1` makes `make upgrade` pull versioned images instead of building |
| `BACKUP_DIR` | Backup directory, default `./backups` |
| `BACKUP_LOG_DIR` | Scheduled-backup log directory, default `BACKUP_DIR` |
| `BACKUP_KEEP` | Number of backups to retain, default `14` |

Optional add-ons are inert when unset:

```env
PRICE_PROVIDER=coinpaprika
CHAIN_PROVIDER=public
# CHAIN_PROVIDER=etherscan
# CHAIN_PROVIDER_API_KEY=your-etherscan-key
AGGREGATION_PROVIDER=fake
AGGREGATION_ENCRYPTION_KEY=<base64 32-byte key>
```

`AGGREGATION_ENCRYPTION_KEY` is required only if you store an aggregation
token. Generate one with:

```bash
python3 -c 'import base64,os; print(base64.b64encode(os.urandom(32)).decode())'
```

## Docker Compose

```bash
docker compose up -d --build
docker compose ps
docker compose logs -f backend
```

Open `http://localhost:3000`. The API health endpoint is
`http://localhost:8080/health`; the version is `GET /api/v1/ping`.

Stop without deleting data:

```bash
docker compose down
```

Delete the database volume (destructive):

```bash
make destroy   # asks for confirmation
```

Never use `docker compose down -v` casually; it removes the `db-data` volume.

### Versioned images and pinning

Pushing a release tag (`v*`) runs CI and publishes both images to GHCR:

```text
ghcr.io/andresberejnoi/networth-tracker-backend:<tag>
ghcr.io/andresberejnoi/networth-tracker-frontend:<tag>
```

The same images are also published as `:latest`. Pin a release in `.env` or on
the command line:

```bash
IMAGE_TAG=v0.1.20261101 docker compose pull
IMAGE_TAG=v0.1.20261101 docker compose up -d --no-build
```

Use `--no-build` so Compose does not replace the pulled image with a local
build. If the GHCR packages are private, log in first with a token that has
`read:packages`:

```bash
echo "$GHCR_TOKEN" | docker login ghcr.io -u YOUR_GITHUB_USER --password-stdin
```

For a local build, use a throwaway tag:

```bash
IMAGE_TAG=dev docker compose up -d --build
```

The published frontend image is compiled with
`VITE_API_BASE_URL=http://localhost:8080`. If you use a different host or port,
build the frontend locally so the URL is compiled into the bundle, or override
the build argument in Compose.

`make upgrade` builds locally by default. To upgrade a pinned deployment from
published images, set `UPGRADE_PULL=1` in `.env` (or the environment); the
script still takes the pre-upgrade backup and checks `/health`:

```bash
IMAGE_TAG=v0.1.20261101 UPGRADE_PULL=1 make upgrade
```

A previous image tag can be used for rollback only while the database schema is
still compatible. Migrations are forward-only; if the newer release applied a
migration, restore the pre-upgrade backup instead of running an older image.

## Podman

The same `docker-compose.yml` works with Podman's Compose provider.

### Rootless Podman

```bash
podman compose up -d --build
podman compose ps
podman compose logs -f backend
```

If `podman compose` is not available, install `podman-compose` and use:

```bash
podman-compose up -d --build
```

For the operational scripts (`make backup`, `make upgrade`, etc.), put the
command in `.env`:

```env
COMPOSE_CMD=podman compose
# or: COMPOSE_CMD=podman-compose
```

For `make up`/`make down`, set the Make variable as well:

```bash
make COMPOSE="podman compose" up
make COMPOSE="podman compose" down
```

The scripts source `.env`; `make up`/`down` use the `COMPOSE` variable.

### Docker compatibility shim

If your system provides `podman-docker`, `docker compose` may already be
emulated by Podman. Enable the user socket if needed:

```bash
systemctl --user enable --now podman.socket
```

This is convenient for the Makefile and scripts, but pure `podman compose` is
fine too.

### SELinux

The stack uses a named volume, so no host bind-mount label is needed. If you add
a host bind mount, append `:Z` to the volume entry.

### Podman on macOS / Windows

Start the Podman machine first:

```bash
podman machine init   # once
podman machine start
podman compose up -d --build
```

Ports are forwarded from the VM; use `localhost:3000` and `localhost:8080`.
Backups written inside the VM should be copied to the host if you want them
outside the machine.

### Start on boot

`restart: unless-stopped` keeps containers running while the Podman service is
up. For a user-level systemd unit, create
`~/.config/systemd/user/networth-tracker.service`:

```ini
[Unit]
Description=NetworthTracker Compose stack
Requires=podman.socket
After=network-online.target

[Service]
Type=oneshot
RemainAfterExit=yes
WorkingDirectory=%h/projects/NetworthTracker
Environment=COMPOSE_CMD=podman compose
ExecStart=/usr/bin/podman compose up -d
ExecStop=/usr/bin/podman compose down

[Install]
WantedBy=default.target
```

Adjust the path and Compose command, then:

```bash
systemctl --user daemon-reload
systemctl --user enable --now networth-tracker.service
loginctl enable-linger "$USER"   # if it should run without an active login
```

## Network and security

- Postgres is not published to the host; only the frontend and API ports are.
- For a single-machine setup, bind to loopback by changing the Compose ports to
  `127.0.0.1:${FRONTEND_PORT}:80` and `127.0.0.1:${BACKEND_PORT}:8080`.
- For LAN access, put the frontend behind a reverse proxy (Caddy, Nginx) with
  TLS and authentication; do not expose the backend or database directly.
- `CORS_ALLOWED_ORIGINS` is derived from `FRONTEND_PORT` in Compose. If you put
  the app behind a different hostname, set it explicitly.
- Use a strong `POSTGRES_PASSWORD`, a separate `BACKUP_PASSPHRASE`, and
  full-disk encryption on the host.
- Keep `.env` out of version control; it is gitignored.

## Backups and upgrades

```bash
make backup     # encrypted backup in ./backups
make restore FILE=backups/networth-....sql.gz.gpg
make upgrade    # backup + rebuild + restart + health check
```

With Podman, set `COMPOSE_CMD=podman compose` in `.env` so the scripts use the
right command. The backend mounts `./backups` at `/backups`; the entrypoint
writes a verified encrypted backup there when pending migrations are detected.
The mount uses `:Z` for SELinux; remove `:Z` on platforms that reject it.
Test a restore before relying on backups. Keep the backup passphrase somewhere
other than the backup directory; without it the backups cannot be restored.

For scheduled backups, see [`backup.md`](backup.md). A user-level systemd timer
can be installed with:

```bash
./scripts/install-backup-timer.sh --install
```

Cron users can run `scripts/scheduled-backup.sh` directly; both paths write a
timestamped log and fail closed when encryption is not configured.

Release tags publish versioned images to GHCR; pin `IMAGE_TAG` for a
reproducible deployment. Scheduled and off-site backups are the next
`robust-upgrades` work.

## Troubleshooting

| Symptom | Check |
|---|---|
| Port already in use | Change `FRONTEND_PORT` / `BACKEND_PORT` in `.env` |
| Backend unhealthy | `docker compose logs backend` or `podman compose logs backend` |
| Login not shown | `AUTH_DISABLED` in `.env` |
| Backup refuses to run | Set a real `BACKUP_PASSPHRASE`; placeholders are rejected |
| Podman cannot find compose | Install `podman-compose` or use the Docker shim |
| SELinux denial on a bind mount | Add `:Z` to the mount |
| Migration/upgrade failure | Use the printed backup with `make restore FILE=...` |
