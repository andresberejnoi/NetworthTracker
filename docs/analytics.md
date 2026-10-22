# Analytics

The app is Rust + TypeScript; Python is an optional, read-only companion for
exploration. Nothing in the runtime depends on Python.

## Read-only database access

Prefer a dedicated read-only role so an analysis script cannot modify data:

```sql
CREATE ROLE networth_reader LOGIN PASSWORD 'choose-a-password';
GRANT CONNECT ON DATABASE networth TO networth_reader;
GRANT USAGE ON SCHEMA public TO networth_reader;
GRANT SELECT ON ALL TABLES IN SCHEMA public TO networth_reader;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT ON TABLES TO networth_reader;
```

```bash
psql "postgres://networth_reader:choose-a-password@localhost:5432/networth"
```

The database is on the Compose network; publish it temporarily if you need to
connect from the host, or run the query inside the container:

```bash
docker compose exec db psql -U networth -d networth
```

## Useful queries

Latest balance per account:

```sql
SELECT DISTINCT ON (a.id)
       a.name, a.account_type, b.as_of, b.amount, b.currency
FROM accounts a
JOIN account_balances b ON b.account_id = a.id
WHERE NOT a.archived
ORDER BY a.id, b.as_of DESC;
```

Total net worth by snapshot date:

```sql
SELECT as_of, total, liquid, liquidity, accessible
FROM net_worth_snapshots
WHERE scope = 'household'
ORDER BY as_of;
```

Spending/asset mix by account type:

```sql
SELECT a.account_type, SUM(b.amount) AS balance
FROM accounts a
JOIN LATERAL (
  SELECT amount FROM account_balances
  WHERE account_id = a.id ORDER BY as_of DESC LIMIT 1
) b ON true
WHERE NOT a.archived
GROUP BY a.account_type
ORDER BY balance DESC;
```

## Python example

```python
import os
import psycopg

url = os.environ["DATABASE_URL"]
with psycopg.connect(url) as conn:
    with conn.cursor() as cur:
        cur.execute("""
            SELECT as_of, total, liquid, liquidity, accessible
            FROM net_worth_snapshots
            WHERE scope = 'household'
            ORDER BY as_of
        """)
        for row in cur.fetchall():
            print(row)
```

Amounts come back as `Decimal` (psycopg) or `numeric` strings; never convert
them to `float` for money math.

## Exports instead of SQL

If you would rather not touch the database, use the CSV/XLSX export from the
**Data** view or `GET /api/v1/export`. It is the same data and keeps decimals
exact.
