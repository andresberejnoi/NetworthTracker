import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useCurrentUser } from "../../auth/AuthContext";
import {
  getBudget,
  recordBudgetSpending,
  setBudgetIncome,
  setBudgetPlan,
  type BudgetAllocation,
} from "../../api/client";

const CATEGORIES = [
  "fixed_costs",
  "investments",
  "savings",
  "discretionary",
  "other",
] as const;

type Category = (typeof CATEGORIES)[number];

const LABELS: Record<Category, string> = {
  fixed_costs: "Fixed costs",
  investments: "Investments",
  savings: "Savings",
  discretionary: "Discretionary",
  other: "Other",
};

function formatAmount(amount: string | null | undefined, currency: string): string {
  if (amount === null || amount === undefined) {
    return "—";
  }
  const [whole, fraction] = amount.split(".");
  const grouped = whole.replace(/\B(?=(\d{3})+(?!\d))/g, ",");
  return `${fraction !== undefined ? `${grouped}.${fraction}` : grouped} ${currency}`;
}

function defaultPercentages(): Record<Category, string> {
  return {
    fixed_costs: "",
    investments: "",
    savings: "",
    discretionary: "",
    other: "",
  };
}

export default function BudgetView() {
  const queryClient = useQueryClient();
  const currentUser = useCurrentUser();
  const canWrite = currentUser?.role !== "viewer";
  const [yearInput, setYearInput] = useState(String(new Date().getUTCFullYear()));
  const year = Number(yearInput) || new Date().getUTCFullYear();

  const budget = useQuery({
    queryKey: ["budget", year],
    queryFn: () => getBudget(year),
  });

  const [income, setIncome] = useState("");
  const [percentages, setPercentages] =
    useState<Record<Category, string>>(defaultPercentages);
  const [spendMonth, setSpendMonth] = useState("1");
  const [spendCategory, setSpendCategory] = useState<Category>("fixed_costs");
  const [spendAmount, setSpendAmount] = useState("");

  useEffect(() => {
    if (!budget.data) return;
    setIncome(budget.data.income ?? "");
    const next = defaultPercentages();
    for (const allocation of budget.data.allocations as BudgetAllocation[]) {
      const category = allocation.category as Category;
      if (CATEGORIES.includes(category)) {
        next[category] = allocation.percentage;
      }
    }
    setPercentages(next);
  }, [budget.data]);

  const refresh = () => {
    queryClient.invalidateQueries({ queryKey: ["budget", year] });
  };

  const saveIncome = useMutation({
    mutationFn: () => setBudgetIncome({ year, amount: income }),
    onSuccess: refresh,
  });

  const savePlan = useMutation({
    mutationFn: () =>
      setBudgetPlan({
        year,
        allocations: CATEGORIES.filter((category) => percentages[category] !== "").map(
          (category) => ({
            category,
            percentage: percentages[category],
          }),
        ),
      }),
    onSuccess: refresh,
  });

  const saveSpending = useMutation({
    mutationFn: () =>
      recordBudgetSpending({
        year,
        month: Number(spendMonth) || 1,
        category: spendCategory,
        amount: spendAmount,
      }),
    onSuccess: () => {
      setSpendAmount("");
      refresh();
    },
  });

  const currency = budget.data?.currency ?? "USD";

  return (
    <section aria-label="budget">
      <section className="card" aria-label="budget summary">
        <h2>Budget</h2>
        <p className="muted">
          Income and spending plans are optional and per user. Amounts are exact
          decimals; no budget simply leaves this area empty.
        </p>
        <div className="controls">
          <label>
            <span>Year</span>
            <input
              aria-label="budget year"
              value={yearInput}
              onChange={(event) => setYearInput(event.target.value)}
            />
          </label>
          {budget.data && (
            <>
              <span className="badge">
                {budget.data.income
                  ? `income ${formatAmount(budget.data.income, currency)}`
                  : "no income configured"}
              </span>
              <span className={budget.data.over_allocated ? "badge badge-warn" : "badge"}>
                {budget.data.over_allocated
                  ? `over-allocated ${budget.data.total_percentage}%`
                  : budget.data.balanced
                    ? "balanced 100%"
                    : `allocated ${budget.data.total_percentage}%`}
              </span>
            </>
          )}
        </div>
        {budget.isPending && <p className="muted">Loading budget …</p>}
        {budget.isError && (
          <p className="error" role="alert">
            {(budget.error as Error).message}
          </p>
        )}
      </section>

      <section className="card" aria-label="budget income">
        <h2>Annual income</h2>
        <form
          className="row"
          onSubmit={(event) => {
            event.preventDefault();
            saveIncome.mutate();
          }}
        >
          <label>
            <span>Income</span>
            <input
              aria-label="budget income"
              placeholder="120000.00"
              value={income}
              onChange={(event) => setIncome(event.target.value)}
            />
          </label>
          <button type="submit" disabled={!canWrite || saveIncome.isPending}>
            Save income
          </button>
        </form>
        {saveIncome.isError && (
          <p className="error" role="alert">
            {(saveIncome.error as Error).message}
          </p>
        )}
      </section>

      <section className="card" aria-label="budget plan">
        <h2>Spending plan</h2>
        <p className="muted">
          Percentages of annual income. A total above 100% is saved but flagged
          as over-allocated.
        </p>
        <form
          className="row"
          onSubmit={(event) => {
            event.preventDefault();
            savePlan.mutate();
          }}
        >
          {CATEGORIES.map((category) => (
            <label key={category}>
              <span>{LABELS[category]} %</span>
              <input
                aria-label={`${LABELS[category]} percentage`}
                value={percentages[category]}
                onChange={(event) =>
                  setPercentages((current) => ({
                    ...current,
                    [category]: event.target.value,
                  }))
                }
              />
            </label>
          ))}
          <button type="submit" disabled={!canWrite || savePlan.isPending}>
            Save plan
          </button>
        </form>
        {savePlan.isError && (
          <p className="error" role="alert">
            {(savePlan.error as Error).message}
          </p>
        )}
      </section>

      <section className="card" aria-label="budget spending">
        <h2>Record spending</h2>
        <form
          className="row"
          onSubmit={(event) => {
            event.preventDefault();
            saveSpending.mutate();
          }}
        >
          <label>
            <span>Month</span>
            <input
              aria-label="budget spending month"
              value={spendMonth}
              onChange={(event) => setSpendMonth(event.target.value)}
            />
          </label>
          <label>
            <span>Category</span>
            <select
              aria-label="budget spending category"
              value={spendCategory}
              onChange={(event) => setSpendCategory(event.target.value as Category)}
            >
              {CATEGORIES.map((category) => (
                <option key={category} value={category}>
                  {LABELS[category]}
                </option>
              ))}
            </select>
          </label>
          <label>
            <span>Amount</span>
            <input
              aria-label="budget spending amount"
              placeholder="0.00"
              value={spendAmount}
              onChange={(event) => setSpendAmount(event.target.value)}
            />
          </label>
          <button type="submit" disabled={!canWrite || saveSpending.isPending}>
            Record spending
          </button>
        </form>
        {saveSpending.isError && (
          <p className="error" role="alert">
            {(saveSpending.error as Error).message}
          </p>
        )}
      </section>

      <section className="card" aria-label="budget variance">
        <h2>Plan versus actual</h2>
        {budget.data && budget.data.variance.length === 0 && (
          <p className="muted">No plan or spending for this year yet.</p>
        )}
        {budget.data && budget.data.variance.length > 0 && (
          <table className="table">
            <thead>
              <tr>
                <th>Category</th>
                <th>Planned</th>
                <th>Actual</th>
                <th>Variance</th>
                <th>Variance %</th>
              </tr>
            </thead>
            <tbody>
              {budget.data.variance.map((row) => (
                <tr key={row.category}>
                  <td>{LABELS[row.category as Category] ?? row.category}</td>
                  <td>{formatAmount(row.planned, currency)}</td>
                  <td>{formatAmount(row.actual, currency)}</td>
                  <td>{formatAmount(row.variance_amount, currency)}</td>
                  <td>
                    {row.variance_percent === null
                      ? "—"
                      : `${row.variance_percent}%`}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>
    </section>
  );
}
