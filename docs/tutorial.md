# Getting started: a guided tour

This walks through a first useful setup: accounts, balances, net worth, bulk
entry, investments, crypto, budgeting, and backups. Everything works locally
and offline; external providers are optional.

## 1. Start the app

You need Docker with Compose, or Podman with a Compose provider. See
[deployment.md](deployment.md) for Docker/Podman details.

```bash
git clone git@github.com:andresberejnoi/NetworthTracker.git
cd NetworthTracker

cp .env.example .env
# edit .env:
#   POSTGRES_PASSWORD=<strong local password>
#   BACKUP_PASSPHRASE=<strong passphrase; store it separately>
```

Start the stack:

```bash
docker compose up --build
# or: podman compose up --build
```

Open **http://localhost:3000**. The API is at **http://localhost:8080**.
On first run, create the household owner (username + password). If you set
`AUTH_DISABLED=true`, the app runs as a single local owner without a login.

> Backups are encrypted. If you lose `BACKUP_PASSPHRASE`, you cannot restore
> the encrypted backups.

## 2. Add your accounts

Open **Accounts**. Start simple: create one account per category and record its
total value.

Examples:

| Name | Account type |
|---|---|
| Checking | `checking` |
| Savings | `savings` |
| 401(k) | `401k` |
| Roth IRA | `roth_ira` |
| Brokerage | `taxable_brokerage` |
| House | `real_estate` |
| Mortgage | `mortgage` |
| Crypto (total) | `crypto` |

Use the **Add balance** action for each account with a date and exact amount.
Do this for the most recent date first; you can add history later.

## 3. Look at net worth

Open **Home**. You will see:

- **Total** — assets minus liabilities.
- **Liquid** — cash, brokerage, retirement, and crypto.
- **Liquidity** — cash available now.
- **Accessible** — after estimated taxes, penalties, and selling costs.

Open **Assumptions** to adjust age/tax/penalty/selling assumptions and see the
accessible figure. **Take snapshot** stores an immutable point in history; the
chart compares snapshots over time. Changing assumptions later does not rewrite
old snapshots.

## 4. Enter many balances quickly

Open **Accounts → Bulk entry** for a spreadsheet-like grid:

1. Select the account.
2. Paste rows from a spreadsheet, or type with keyboard navigation.
3. Fix any inline errors.
4. Save atomically or allow partial saves.

You can also use **Data → Import** with the CSV/XLSX template. Download the
template, fill it in, and import. Validation runs before any writes, and
re-importing the same file is idempotent.

## 5. Track investments and tax lots

Open **Portfolio → Investments**.

1. Create a security (for example `VTI`) with an asset class and sector.
2. Add a purchase lot: account, acquired date, quantity, total cost basis
   (including fees), and optional fees.
3. Add more lots as needed. Adding a lot **appends**; it does not replace
   existing lots.
4. Set a manual price, or configure an optional price provider.
5. Record a sale to consume lots FIFO. The realized-gains table shows proceeds,
   basis, gain/loss, and short/long term.

Mark historical lots with **estimated** basis if you only know an approximate
purchase price. Calculations use the value you enter; the UI/report flags it.

## 6. Track crypto

Open **Portfolio → Crypto**.

### Manual wallet / bucket

1. Add an asset, for example `BTC` on `bitcoin`.
2. Add a manual wallet, for example `Coinbase`.
3. Record a dated total, for example `0.003`.

Use the same label for the same real wallet; use distinct labels for distinct
wallets (`Coinbase BTC 1`, `Coinbase BTC 2`). Multiple wallets of the same asset
aggregate into one BTC position.

### Address wallet

1. Add an address wallet with a public address. Private keys and seed phrases
   are rejected.
2. If a chain provider is configured, click **Sync**.
3. Without a provider, record a manual balance; the app remains fully usable
   offline.

### Totals versus purchase lots

A total-only position is fine. If you later want tax lots:

1. Select the wallet in **Crypto tax lots**.
2. Click **Use balance as opening lot**.
3. Enter the opening cost basis (or mark it estimated).
4. Add later purchases as separate lots.

Once an opening lot exists, adding lots or recording a sale updates the wallet
balance to the remaining lots total. If lots and balance differ, the UI warns
you rather than silently choosing one.

### Prices

Set a manual price in **Manual price**, or enable a free price adapter such as
`PRICE_PROVIDER=coinpaprika`. Manual prices always win; cached prices are marked
stale when offline.

## 7. Budget (optional)

Open **Budget** from the optional sidebar section:

1. Enter annual income.
2. Assign category percentages.
3. Record monthly spending per category.
4. Review planned versus actual variance.

A total above 100% is saved but flagged as over-allocated. If you never set a
budget, the rest of the app is unaffected.

## 8. Back up and upgrade

```bash
make backup     # encrypted backup in ./backups
make restore FILE=backups/networth-....sql.gz.gpg
make upgrade    # backup, rebuild, restart, health check
```

Never run `docker compose down -v` unless you intend to delete the database.
`make down` keeps the volume; `make destroy` wipes it with confirmation.

Test a restore before relying on backups. The full database backup is the
authoritative disaster-recovery path; the JSON backup is portable data.

## 9. Where your data lives

- PostgreSQL data: Docker/Podman volume `networth-tracker_db-data`.
- Backups: `./backups` on the host.
- Secrets: `.env` on the host, gitignored.
- No cloud dependency for accounts, balances, net worth, history, bulk entry,
  import/export, investments, crypto balances, budgets, or manual prices.

## Troubleshooting

- **Port already in use** — set `FRONTEND_PORT`/`BACKEND_PORT` in `.env`.
- **Login not shown** — check `AUTH_DISABLED`; `true` means single-user mode.
- **Backup fails** — set a real `BACKUP_PASSPHRASE`; the placeholder is refused.
- **Upgrade fails** — the pre-upgrade backup path is printed; restore it with
  `make restore FILE=...`.
- **Podman** — see [deployment.md](deployment.md) for `COMPOSE_CMD`, the
  Podman socket, and rootless notes.
