-- Income, spending plans, and category spending.
--
-- Budgeting is optional and per user. A plan stores percentages; allocated
-- amounts are derived from the user's income for the same year.

CREATE TABLE IF NOT EXISTS budget_incomes (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL,
    year        INTEGER NOT NULL,
    amount      NUMERIC NOT NULL,
    currency    TEXT NOT NULL DEFAULT 'USD',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT budget_incomes_year CHECK (year BETWEEN 1900 AND 2200),
    CONSTRAINT budget_incomes_amount_non_negative CHECK (amount >= 0),
    CONSTRAINT budget_incomes_currency CHECK (currency ~ '^[A-Z]{3}$'),
    CONSTRAINT budget_incomes_one_per_year UNIQUE (user_id, year)
);

CREATE TABLE IF NOT EXISTS budget_plans (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL,
    year        INTEGER NOT NULL,
    currency    TEXT NOT NULL DEFAULT 'USD',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT budget_plans_year CHECK (year BETWEEN 1900 AND 2200),
    CONSTRAINT budget_plans_currency CHECK (currency ~ '^[A-Z]{3}$'),
    CONSTRAINT budget_plans_one_per_year UNIQUE (user_id, year)
);

CREATE TABLE IF NOT EXISTS budget_allocations (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    plan_id     UUID NOT NULL REFERENCES budget_plans (id) ON DELETE CASCADE,
    category    TEXT NOT NULL,
    percentage  NUMERIC NOT NULL,
    CONSTRAINT budget_allocations_category CHECK (category IN (
        'fixed_costs', 'investments', 'savings', 'discretionary', 'other'
    )),
    CONSTRAINT budget_allocations_percentage CHECK (
        percentage >= 0 AND percentage <= 100
    ),
    CONSTRAINT budget_allocations_unique_category UNIQUE (plan_id, category)
);

CREATE TABLE IF NOT EXISTS budget_spending (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL,
    year        INTEGER NOT NULL,
    month       INTEGER NOT NULL,
    category    TEXT NOT NULL,
    amount      NUMERIC NOT NULL,
    currency    TEXT NOT NULL DEFAULT 'USD',
    note        TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT budget_spending_year CHECK (year BETWEEN 1900 AND 2200),
    CONSTRAINT budget_spending_month CHECK (month BETWEEN 1 AND 12),
    CONSTRAINT budget_spending_category CHECK (category IN (
        'fixed_costs', 'investments', 'savings', 'discretionary', 'other'
    )),
    CONSTRAINT budget_spending_amount_non_negative CHECK (amount >= 0),
    CONSTRAINT budget_spending_currency CHECK (currency ~ '^[A-Z]{3}$'),
    CONSTRAINT budget_spending_one_per_category_month UNIQUE (
        user_id, year, month, category
    )
);
