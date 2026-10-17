import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  ACCOUNT_TYPES,
  archiveAccount,
  createAccount,
  listAccounts,
  type AccountType,
} from "../../api/client";

export default function AccountsView() {
  const queryClient = useQueryClient();
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

  return (
    <section className="card" aria-label="accounts">
      <h2>Accounts</h2>

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

        <button type="submit" disabled={create.isPending || name.trim() === ""}>
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
                  >
                    Archive
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}
