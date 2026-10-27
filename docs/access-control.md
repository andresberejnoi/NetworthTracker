# Access control

NetworthTracker supports a family/household with multiple local users and
role-based access. Everything stays on the machine.

## Auth modes

- **Enabled (default).** On first run the app asks you to create the household
  owner. After that, users sign in with a username and password.
- **Disabled.** Set `AUTH_DISABLED=true` for a single-user offline setup with no
  login. Every request acts as an implicit local owner.

```yaml
# docker-compose.yml
environment:
  AUTH_DISABLED: "false"   # or "true" for no login
```

## Roles

| Role | Read | Write data | Manage users | See all private accounts |
|------|------|------------|--------------|--------------------------|
| `owner` | ✅ | ✅ | ✅ | ✅ |
| `editor` | ✅ | ✅ | ❌ | ❌ |
| `viewer` | ✅ | ❌ | ❌ | ❌ |

- **Write** covers accounts, balances, bulk entry, imports, and snapshots.
- A `viewer` sees the UI with mutation controls disabled.
- Account visibility: household-shared accounts are visible to every member;
  a `user`-scoped account is visible only to its owner (and to the household
  owner). Another member's private account returns `404`, so its existence is
  not leaked.
- Net worth respects the same visibility: a non-owner's household view includes
  shared accounts plus their own private accounts, never someone else's.

## Sessions

- Passwords are hashed with **Argon2id**.
- `POST /api/v1/auth/login` returns a bearer token; only its **SHA-256 hash** is
  stored in the database.
- Sessions last 30 days. `POST /api/v1/auth/logout` deletes the session.
- The frontend keeps the token in `localStorage` and sends
  `Authorization: Bearer <token>`. That is simple and works with the decoupled
  frontend, but it is readable by JavaScript on the page; on a shared machine,
  rely on OS/disk encryption as well.

## API

```bash
# Is auth on? Is first-run setup needed?
curl -s localhost:8080/api/v1/auth/status

# First run only
curl -s -X POST localhost:8080/api/v1/auth/setup \
  -H 'content-type: application/json' \
  -d '{"username":"owner","password":"a-long-password","display_name":"Owner"}'

# Sign in
curl -s -X POST localhost:8080/api/v1/auth/login \
  -H 'content-type: application/json' \
  -d '{"username":"owner","password":"a-long-password"}'

# Authenticated requests
curl -s localhost:8080/api/v1/auth/me -H "Authorization: Bearer $TOKEN"
curl -s localhost:8080/api/v1/accounts -H "Authorization: Bearer $TOKEN"

# Owner-only user management
curl -s localhost:8080/api/v1/users -H "Authorization: Bearer $TOKEN"
curl -s -X POST localhost:8080/api/v1/users \
  -H "Authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d '{"username":"kid","password":"a-long-password","role":"viewer"}'
```

| Status | Meaning |
|--------|---------|
| 401 | Missing or invalid session |
| 403 | Authenticated but the role does not allow it |
| 404 | Resource missing **or** hidden by per-user scoping |

## Frontend

- The app checks `/api/v1/auth/status` on load and shows either the setup
  screen, the login screen, or the app.
- The header shows the signed-in member and their role, with **Sign out** when
  auth is enabled.
- **Users** appears in the nav for owners only.
- A viewer sees read-only screens: account/balance mutations, bulk save, import,
  and snapshot creation are disabled.

## Security notes

- This is a local threat model: the database and the machine are the boundary.
  Use full-disk encryption if the machine is shared or portable.
- Do not expose the API or frontend to the internet without TLS and a real
  reverse proxy.
- `AUTH_DISABLED=true` is convenient for a trusted single-user machine; it is
  not a substitute for authentication on a shared one.
