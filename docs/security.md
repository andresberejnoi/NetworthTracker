# Security

## Secret handling rules

- Never commit real passwords, API keys, tokens, private keys, or `.env` files.
- Local secrets live in `.env` (gitignored). `.env.example` documents the keys
  with placeholders.
- Use obvious placeholders in docs and clearly fake values in tests.
- If a credential is ever committed, **rotate it first**, then remove it from
  history. Deleting the file is not enough.
- Provider credentials (prices, chains, aggregation) are supplied by the
  individual user and stored locally; the product bundles none and runs no
  central relay.

## gitleaks

[gitleaks](https://github.com/gitleaks/gitleaks) scans git history, the working
tree, and staged changes for hardcoded secrets using maintained regex rules and
entropy checks.

### Install

Download the release binary for your platform from the gitleaks releases page,
or use your package manager if it packages it. Put it on `PATH` or at
`~/.local/bin/gitleaks` (the runner checks both).

### Enable the pre-commit hook

```bash
make hooks
```

This sets `core.hooksPath` to `.githooks`. From then on, every commit runs:

1. `scripts/check-secrets.sh` — a fast scan of tracked/staged files
2. `gitleaks git --staged` — a deep scan of the staged diff

The hook fails closed: if gitleaks is not installed, the commit is blocked with
an install message.

### Run it manually

```bash
make secrets                                  # fast scan + full history
./scripts/run-gitleaks.sh git                 # full history
./scripts/run-gitleaks.sh git --staged        # staged diff only
./scripts/run-gitleaks.sh dir .               # working tree
```

### Allowlist policy

`.gitleaks.toml` extends the default rules and allowlists **only** these
pre-release, fake values, each anchored to an exact match:

- `password123` — an old test fixture
- the old local Compose default that used `networth` as both the database user
  and password
- the same default in YAML form

The anchors matter: a real secret that merely *contains* `password123` is still
reported. Nothing else may be allowlisted. Never allowlist a real credential;
rotate it instead. If a new false positive appears, prefer changing the value to
an obvious placeholder over widening the allowlist.

For zero allowlist, rewrite history before the first release tag to remove the
historic values entirely. The repository is pre-release, so that is still
possible.

No scanner is exhaustive. gitleaks catches known token formats and
high-entropy strings; it cannot catch every possible password. The allowlist is
a narrow, documented exception, not a guarantee.

### CI

CI installs gitleaks, runs `gitleaks git` over the full history with the same
config, and uploads the SARIF report. A finding fails the job.

## Authentication and sessions

- Passwords are hashed with **Argon2id**.
- Session tokens are random; only their SHA-256 hash is stored. Sessions expire
  after 30 days and can be revoked with `POST /api/v1/auth/logout`.
- The frontend keeps the token in `localStorage` and sends it as a bearer token.
  This is simple and works with the decoupled frontend, but page JavaScript can
  read it. For a shared or portable machine, use full-disk encryption and do not
  expose the app to the internet without TLS and a reverse proxy.
- `AUTH_DISABLED=true` is for a trusted single-user machine only.

## Database

- The database password comes from `.env`; there is no committed default.
- The container listens on the Compose network only. Do not publish Postgres to
  the host or the internet.
- Use `pg_dump` or the JSON backup endpoint for backups, and protect the backup
  files like any other secret.

## Reporting a leak

1. Rotate the affected credential at the provider.
2. Remove it from the code and, if needed, from git history.
3. Run `make secrets` and `gitleaks git` to confirm.
4. Record the incident and the rotation in your notes.
