import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useCurrentUser } from "../../auth/AuthContext";
import {
  ACCOUNT_TYPES,
  archiveAccount,
  createAccount,
  listAccounts,
  recordBalance,
  type AccountType,
} from "../../api/client";

export default function AccountsView() {
  const queryClient = useQueryClient();
  const currentUser = useCurrentUser();
  const canWrite = currentUser?.role !== "viewer";
  const [name, setName] = useState("");
  const [accountType, setAccountType] = useState<AccountType>("checking");
  const [institution, setInstitution] = useState("");

  const accounts = useQuery({
    queryKey: ["accounts"],
    queryFn: () => listAccounts(),
  });

  const create = useMutation({
    mutationFn: () =>
      createAccount({
        name,
        account_type: accountType,
        institution: institution.trim() === "" ? null : institution.trim(),
      }),
    onSuccess: () => {
      setName("");
      setInstitution("");
      queryClient.invalidateQueries({ queryKey: ["accounts"] });
    },
  });

  const archive = useMutation({
    mutationFn: (id: string) => archiveAccount(id),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["accounts"] }),
  });

  const [balanceAccount, setBalanceAccount] = useState("");
  const [balanceDate, setBalanceDate] = useState(
    new Date().toISOString().slice(0, 10),
  );
  const [balanceAmount, setBalanceAmount] = useState("");

  const addBalance = useMutation({
    mutationFn: () =>
      recordBalance(balanceAccount, {
        as_of: balanceDate,
        amount: balanceAmount,
      }),
    onSuccess: () => {
      setBalanceAmount("");
      queryClient.invalidateQueries({ queryKey: ["accounts"] });
      queryClient.invalidateQueries({ queryKey: ["networth"] });
    },
  });

  return (
    <section className="card" aria-label="accounts">
      <h2>Accounts</h2>

      {!canWrite && (
        <p className="muted">Your role is read-only, so changes are disabled.</p>
      )}

      <form
        className="row"
        onSubmit={(event) => {
          event.preventDefault();
          create.mutate();
        }}
      >
        <label>
          <span>Name</span>
          <input
            aria-label="account name"
            value={name}
            onChange={(event) => setName(event.target.value)}
            placeholder="Fidelity 401k"
          />
        </label>

        <label>
          <span>Type</span>
          <select
            aria-label="account type"
            value={accountType}
            onChange={(event) => setAccountType(event.target.value as AccountType)}
          >
            {ACCOUNT_TYPES.map((type) => (
              <option key={type} value={type}>
                {type}
              </option>
            ))}
          </select>
        </label>

        <label>
          <span>Institution</span>
          <input
            aria-label="institution"
            value={institution}
            onChange={(event) => setInstitution(event.target.value)}
            placeholder="optional"
          />
        </label>

        <button type="submit" disabled={create.isPending || name.trim() === "" || !canWrite}>
          Add account
        </button>
      </form>

      {create.isError && (
        <p className="error" role="alert">
          {(create.error as Error).message}
        </p>
      )}

      {accounts.isPending && <p className="muted">Loading accounts …</p>}
      {accounts.isError && (
        <p className="error" role="alert">
          {(accounts.error as Error).message}
        </p>
      )}

      {accounts.data && accounts.data.length === 0 && (
        <p className="muted">No accounts yet — add your first one above.</p>
      )}

      {accounts.data && accounts.data.length > 0 && (
        <table className="table">
          <thead>
            <tr>
              <th>Name</th>
              <th>Type</th>
              <th>Institution</th>
              <th aria-label="actions" />
            </tr>
          </thead>
          <tbody>
            {accounts.data.map((account) => (
              <tr key={account.id}>
                <td>{account.name}</td>
                <td>{account.account_type}</td>
                <td>{account.institution ?? "—"}</td>
                <td>
                  <button
                    type="button"
                    onClick={() => archive.mutate(account.id)}
                    aria-label={`archive ${account.name}`}
                    disabled={!canWrite}
                  >
                    Archive
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      {accounts.data && accounts.data.length > 0 && (
        <>
          <h3>Record a balance</h3>
          <p className="muted">
            For simple tracking, create one account per category (for example
            “Stocks (total)” or “Real estate (total)”) and record its total
            value here. No per-instrument data is required.
          </p>
          <form
            className="row"
            onSubmit={(event) => {
              event.preventDefault();
              addBalance.mutate();
            }}
          >
            <label>
              <span>Account</span>
              <select
                aria-label="balance account"
                value={balanceAccount}
                onChange={(event) => setBalanceAccount(event.target.value)}
              >
                <option value="">Select …</option>
                {accounts.data.map((account) => (
                  <option key={account.id} value={account.id}>
                    {account.name}
                  </option>
                ))}
              </select>
            </label>
            <label>
              <span>Date</span>
              <input
                aria-label="balance date"
                type="date"
                value={balanceDate}
                onChange={(event) => setBalanceDate(event.target.value)}
              />
            </label>
            <label>
              <span>Amount</span>
              <input
                aria-label="balance amount"
                value={balanceAmount}
                onChange={(event) => setBalanceAmount(event.target.value)}
                placeholder="0.00"
              />
            </label>
            <button
              type="submit"
              disabled={
                !canWrite ||
                addBalance.isPending ||
                balanceAccount === "" ||
                balanceAmount.trim() === ""
              }
            >
              Save balance
            </button>
          </form>
          {addBalance.isError && (
            <p className="error" role="alert">
              {(addBalance.error as Error).message}
            </p>
          )}
          {addBalance.isSuccess && <p className="muted">Balance saved.</p>}
        </>
      )}
    </section>
  );
}
