# Budgeting

Budgeting is optional and per user. If no income or plan exists, the rest of
the app works normally and the Budget view is simply empty.

## Model

- **Annual income** — one amount per user/year, used as the plan basis.
- **Spending plan** — percentages across `fixed_costs`, `investments`,
  `savings`, `discretionary`, and `other`.
- **Spending record** — one actual amount per user/year/month/category.
- **Variance** — planned amount, actual amount, difference, and difference
  percentage per category.

Planned amount = annual income × category percentage / 100. A plan whose
percentages total exactly 100 is **balanced**; above 100 is **over-allocated**
and is flagged but saved so the user can see the problem.

## API

```bash
# annual income
curl -s -X POST localhost:8080/api/v1/budget/income \
  -H "Authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d '{"year":2026,"amount":"120000.00"}'

# plan
curl -s -X POST localhost:8080/api/v1/budget/plan \
  -H "Authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d '{"year":2026,"allocations":[
        {"category":"fixed_costs","percentage":"50"},
        {"category":"investments","percentage":"20"},
        {"category":"savings","percentage":"10"},
        {"category":"discretionary","percentage":"15"},
        {"category":"other","percentage":"5"}]}'

# actual spending
curl -s -X POST localhost:8080/api/v1/budget/spending \
  -H "Authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d '{"year":2026,"month":1,"category":"fixed_costs","amount":"1000.00"}'

# full view, including variance
curl -s 'localhost:8080/api/v1/budget?year=2026' \
  -H "Authorization: Bearer $TOKEN"
```

All amounts are exact decimals and cross JSON as strings. Viewers can read the
budget but cannot change it.
