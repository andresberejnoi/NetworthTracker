import { useState } from "react";
import { Link, useParams } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { useCurrentUser } from "../../auth/AuthContext";
import { getAccount, listBalances } from "../../api/client";
import AddBalanceDrawer from "../../components/AddBalanceDrawer";
import DataTable from "../../components/DataTable";
import EmptyState from "../../components/EmptyState";
import PageHeader from "../../components/PageHeader";

export default function AccountDetailView() {
  const { id = "" } = useParams();
  const currentUser = useCurrentUser();
  const canWrite = currentUser?.role !== "viewer";
  const [balanceOpen, setBalanceOpen] = useState(false);

  const account = useQuery({
    queryKey: ["account", id],
    queryFn: () => getAccount(id),
    enabled: id !== "",
  });
  const balances = useQuery({
    queryKey: ["balances", id],
    queryFn: () => listBalances(id),
    enabled: id !== "",
  });

  if (account.isError) {
    return (
      <section>
        <PageHeader title="Account" />
        <p className="error" role="alert">
          {(account.error as Error).message}
        </p>
        <Link to="/accounts">Back to accounts</Link>
      </section>
    );
  }

  return (
    <section>
      <PageHeader
        title={account.data?.name ?? "Account"}
        description={
          account.data
            ? `${account.data.account_type}${
                account.data.institution ? ` · ${account.data.institution}` : ""
              }`
            : "Loading account…"
        }
        actions={
          <>
            <button
              type="button"
              onClick={() => setBalanceOpen(true)}
              disabled={!canWrite || !account.data}
            >
              Add balance
            </button>
            <Link className="button-link secondary" to="/accounts">
              Back to accounts
            </Link>
          </>
        }
      />

      <section className="card" aria-label="account balances">
        <h2>Balance history</h2>
        {balances.isPending && <p className="muted">Loading balances…</p>}
        {balances.isError && (
          <p className="error" role="alert">
            {(balances.error as Error).message}
          </p>
        )}
        {balances.data && balances.data.length === 0 && (
          <EmptyState
            title="No balances recorded"
            description="Record the current balance to start this account's history."
            action={
              <button
                type="button"
                disabled={!canWrite}
                onClick={() => setBalanceOpen(true)}
              >
                Add balance
              </button>
            }
          />
        )}
        {balances.data && balances.data.length > 0 && (
          <DataTable>
            <thead>
              <tr>
                <th>Date</th>
                <th>Amount</th>
                <th>Source</th>
              </tr>
            </thead>
            <tbody>
              {balances.data.map((balance) => (
                <tr key={balance.as_of}>
                  <td>{balance.as_of}</td>
                  <td>
                    {balance.amount} {balance.currency}
                  </td>
                  <td>{balance.source}</td>
                </tr>
              ))}
            </tbody>
          </DataTable>
        )}
      </section>

      <AddBalanceDrawer
        open={balanceOpen}
        onClose={() => setBalanceOpen(false)}
        initialAccountId={id}
      />
    </section>
  );
}
