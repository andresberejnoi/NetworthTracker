import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { listAccounts, recordBalance } from "../api/client";
import Drawer from "./Drawer";
import FormField from "./FormField";

interface AddBalanceDrawerProps {
  open: boolean;
  onClose: () => void;
  initialAccountId?: string;
}

function today(): string {
  return new Date().toISOString().slice(0, 10);
}

export default function AddBalanceDrawer({
  open,
  onClose,
  initialAccountId = "",
}: AddBalanceDrawerProps) {
  const queryClient = useQueryClient();
  const accounts = useQuery({
    queryKey: ["accounts"],
    queryFn: () => listAccounts(),
    enabled: open,
  });
  const [accountId, setAccountId] = useState(initialAccountId);
  const [asOf, setAsOf] = useState(today());
  const [amount, setAmount] = useState("");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (open) {
      setAccountId(initialAccountId);
      setAsOf(today());
      setAmount("");
      setError(null);
    }
  }, [open, initialAccountId]);

  const save = useMutation({
    mutationFn: () =>
      recordBalance(accountId, {
        as_of: asOf,
        amount,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["accounts"] });
      queryClient.invalidateQueries({ queryKey: ["networth"] });
      onClose();
    },
    onError: (saveError) => setError((saveError as Error).message),
  });

  if (!open) return null;

  return (
    <Drawer open={open} title="Add balance" onClose={onClose}>
      <form
        className="stack"
        onSubmit={(event) => {
          event.preventDefault();
          setError(null);
          save.mutate();
        }}
      >
        <FormField label="Account">
          <select
            aria-label="balance account"
            value={accountId}
            onChange={(event) => setAccountId(event.target.value)}
          >
            <option value="">Select an account…</option>
            {(accounts.data ?? []).map((account) => (
              <option key={account.id} value={account.id}>
                {account.name}
              </option>
            ))}
          </select>
        </FormField>
        <FormField label="Date">
          <input
            aria-label="balance date"
            type="date"
            value={asOf}
            onChange={(event) => setAsOf(event.target.value)}
          />
        </FormField>
        <FormField
          label="Amount"
          hint="Exact decimal text, for example 1234.56"
        >
          <input
            aria-label="balance amount"
            inputMode="decimal"
            value={amount}
            onChange={(event) => setAmount(event.target.value)}
            placeholder="0.00"
          />
        </FormField>
        {accounts.isError && (
          <p className="error" role="alert">
            {(accounts.error as Error).message}
          </p>
        )}
        {error && (
          <p className="error" role="alert">
            {error}
          </p>
        )}
        <div className="form-actions">
          <button
            type="submit"
            disabled={
              save.isPending ||
              accountId === "" ||
              amount.trim() === "" ||
              asOf === ""
            }
          >
            {save.isPending ? "Saving…" : "Save balance"}
          </button>
          <button type="button" className="secondary" onClick={onClose}>
            Cancel
          </button>
        </div>
      </form>
    </Drawer>
  );
}
