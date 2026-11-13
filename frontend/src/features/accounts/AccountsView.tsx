import { useState } from "react";
import { Link } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useCurrentUser } from "../../auth/AuthContext";
import {
  ACCOUNT_TYPES,
  archiveAccount,
  createAccount,
  listAccounts,
  type Account,
  type AccountType,
} from "../../api/client";
import AddBalanceDrawer from "../../components/AddBalanceDrawer";
import DataTable from "../../components/DataTable";
import Drawer from "../../components/Drawer";
import EmptyState from "../../components/EmptyState";
import FormField from "../../components/FormField";
import PageHeader from "../../components/PageHeader";

export default function AccountsView() {
  const queryClient = useQueryClient();
  const currentUser = useCurrentUser();
  const canWrite = currentUser?.role !== "viewer";
  const [createOpen, setCreateOpen] = useState(false);
  const [balanceOpen, setBalanceOpen] = useState(false);
  const [balanceAccount, setBalanceAccount] = useState("");
  const [name, setName] = useState("");
  const [accountType, setAccountType] = useState<AccountType>("checking");
  const [institution, setInstitution] = useState("");
  const [search, setSearch] = useState("");

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
      setCreateOpen(false);
      queryClient.invalidateQueries({ queryKey: ["accounts"] });
      queryClient.invalidateQueries({ queryKey: ["networth"] });
    },
  });

  const archive = useMutation({
    mutationFn: (id: string) => archiveAccount(id),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["accounts"] }),
  });

  const visibleAccounts = (accounts.data ?? []).filter((account) => {
    const query = search.trim().toLowerCase();
    if (query === "") return true;
    return (
      account.name.toLowerCase().includes(query) ||
      account.account_type.toLowerCase().includes(query) ||
      (account.institution ?? "").toLowerCase().includes(query)
    );
  });

  const openBalance = (account?: Account) => {
    setBalanceAccount(account?.id ?? "");
    setBalanceOpen(true);
  };

  return (
    <section aria-label="accounts">
      <PageHeader
        title="Accounts"
        description="Track balances for each account. Add a balance whenever it changes."
        actions={
          <>
            <button
              type="button"
              onClick={() => setCreateOpen(true)}
              disabled={!canWrite}
            >
              Add account
            </button>
            <button
              type="button"
              className="secondary"
              onClick={() => openBalance()}
              disabled={!canWrite || (accounts.data?.length ?? 0) === 0}
            >
              Add balance
            </button>
            <Link className="button-link secondary" to="/accounts/bulk">
              Bulk entry
            </Link>
          </>
        }
      />

      {!canWrite && (
        <p className="muted">Your role is read-only, so changes are disabled.</p>
      )}

      {create.isError && (
        <p className="error" role="alert">
          {(create.error as Error).message}
        </p>
      )}
      {archive.isError && (
        <p className="error" role="alert">
          {(archive.error as Error).message}
        </p>
      )}

      {accounts.isPending && <p className="muted">Loading accounts …</p>}
      {accounts.isError && (
        <p className="error" role="alert">
          {(accounts.error as Error).message}
        </p>
      )}

      {accounts.data && accounts.data.length === 0 && (
        <EmptyState
          title="No accounts yet"
          description="Add your first checking, savings, brokerage, retirement, or property account."
          action={
            <button
              type="button"
              disabled={!canWrite}
              onClick={() => setCreateOpen(true)}
            >
              Add your first account
            </button>
          }
        />
      )}

      {accounts.data && accounts.data.length > 0 && (
        <div className="controls">
          <label>
            <span>Search accounts</span>
            <input
              aria-label="search accounts"
              value={search}
              onChange={(event) => setSearch(event.target.value)}
              placeholder="Name, type, or institution"
            />
          </label>
        </div>
      )}

      {accounts.data && accounts.data.length > 0 && visibleAccounts.length === 0 && (
        <EmptyState
          title="No matching accounts"
          description="Try a different name, type, or institution."
        />
      )}

      {accounts.data && accounts.data.length > 0 && visibleAccounts.length > 0 && (
        <DataTable>
          <thead>
            <tr>
              <th>Name</th>
              <th>Type</th>
              <th>Institution</th>
              <th aria-label="actions" />
            </tr>
          </thead>
          <tbody>
            {visibleAccounts.map((account) => (
              <tr key={account.id}>
                <td>
                  <Link to={`/accounts/${account.id}`}>{account.name}</Link>
                </td>
                <td>{account.account_type}</td>
                <td>{account.institution ?? "—"}</td>
                <td className="row-actions">
                  <button
                    type="button"
                    onClick={() => openBalance(account)}
                    disabled={!canWrite}
                  >
                    Add balance
                  </button>
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
        </DataTable>
      )}

      <Drawer
        open={createOpen}
        title="Add account"
        onClose={() => setCreateOpen(false)}
      >
        <form
          className="stack"
          onSubmit={(event) => {
            event.preventDefault();
            create.mutate();
          }}
        >
          <FormField label="Name">
            <input
              aria-label="account name"
              value={name}
              onChange={(event) => setName(event.target.value)}
              placeholder="Fidelity 401k"
            />
          </FormField>
          <FormField label="Type">
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
          </FormField>
          <FormField label="Institution" hint="Optional">
            <input
              aria-label="institution"
              value={institution}
              onChange={(event) => setInstitution(event.target.value)}
              placeholder="Fidelity"
            />
          </FormField>
          <div className="form-actions">
            <button
              type="submit"
              disabled={create.isPending || name.trim() === "" || !canWrite}
            >
              {create.isPending ? "Creating…" : "Create account"}
            </button>
            <button
              type="button"
              className="secondary"
              onClick={() => setCreateOpen(false)}
            >
              Cancel
            </button>
          </div>
        </form>
      </Drawer>

      <AddBalanceDrawer
        open={balanceOpen}
        onClose={() => setBalanceOpen(false)}
        initialAccountId={balanceAccount}
      />
    </section>
  );
}
