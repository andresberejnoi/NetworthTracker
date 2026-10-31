import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useCurrentUser } from "../../auth/AuthContext";
import {
  createCryptoAsset,
  createCryptoWallet,
  getChainProvider,
  listCryptoAssets,
  listCryptoPositions,
  listCryptoWallets,
  recordCryptoBalance,
  setPrice,
  syncCryptoWallet,
  type CryptoWallet,
} from "../../api/client";
import BulkGrid, {
  type GridColumn,
  type GridRow,
  type SaveResult,
} from "../bulk-entry/BulkGrid";

const CHAINS = ["bitcoin", "ethereum"] as const;

const BULK_COLUMNS: GridColumn[] = [
  {
    key: "asset",
    label: "Asset",
    placeholder: "BTC",
    validate: (value) => (value === "" ? "asset is required" : null),
  },
  {
    key: "label",
    label: "Wallet",
    placeholder: "Coinbase",
    validate: (value) => (value === "" ? "wallet is required" : null),
  },
  {
    key: "kind",
    label: "Kind",
    placeholder: "manual",
  },
  {
    key: "address",
    label: "Address",
    placeholder: "public address (address mode)",
  },
  {
    key: "quantity",
    label: "Quantity",
    placeholder: "manual amount",
  },
  {
    key: "as_of",
    label: "As of",
    placeholder: "YYYY-MM-DD",
  },
];

function formatAmount(amount: string | null | undefined, currency: string): string {
  if (amount === null || amount === undefined) {
    return "—";
  }
  const [whole, fraction] = amount.split(".");
  const grouped = whole.replace(/\B(?=(\d{3})+(?!\d))/g, ",");
  return `${fraction !== undefined ? `${grouped}.${fraction}` : grouped} ${currency}`;
}

export default function CryptoView() {
  const queryClient = useQueryClient();
  const currentUser = useCurrentUser();
  const canWrite = currentUser?.role !== "viewer";

  const assets = useQuery({ queryKey: ["crypto-assets"], queryFn: listCryptoAssets });
  const wallets = useQuery({ queryKey: ["crypto-wallets"], queryFn: listCryptoWallets });
  const positions = useQuery({
    queryKey: ["crypto-positions"],
    queryFn: () => listCryptoPositions(),
  });
  const provider = useQuery({ queryKey: ["chain-provider"], queryFn: getChainProvider });

  const [assetSymbol, setAssetSymbol] = useState("");
  const [assetName, setAssetName] = useState("");
  const [assetChain, setAssetChain] = useState<(typeof CHAINS)[number]>("bitcoin");
  const [assetDecimals, setAssetDecimals] = useState("18");

  const [walletAsset, setWalletAsset] = useState("");
  const [walletKind, setWalletKind] = useState<"manual" | "address">("manual");
  const [walletLabel, setWalletLabel] = useState("");
  const [walletAddress, setWalletAddress] = useState("");

  const [balanceWallet, setBalanceWallet] = useState("");
  const [balanceDate, setBalanceDate] = useState("");
  const [balanceQuantity, setBalanceQuantity] = useState("");

  const [priceSymbol, setPriceSymbol] = useState("");
  const [priceValue, setPriceValue] = useState("");

  const selectedAsset = (assets.data ?? []).find((asset) => asset.id === walletAsset);

  const refresh = () => {
    queryClient.invalidateQueries({ queryKey: ["crypto-assets"] });
    queryClient.invalidateQueries({ queryKey: ["crypto-wallets"] });
    queryClient.invalidateQueries({ queryKey: ["crypto-positions"] });
    queryClient.invalidateQueries({ queryKey: ["networth"] });
  };

  const addAsset = useMutation({
    mutationFn: () =>
      createCryptoAsset({
        symbol: assetSymbol,
        name: assetName,
        chain: assetChain,
        decimals: Number(assetDecimals) || 18,
      }),
    onSuccess: () => {
      setAssetSymbol("");
      setAssetName("");
      refresh();
    },
  });

  const addWallet = useMutation({
    mutationFn: () =>
      createCryptoWallet({
        asset_id: walletAsset,
        kind: walletKind,
        chain: selectedAsset?.chain ?? "bitcoin",
        label: walletLabel,
        address: walletKind === "address" ? walletAddress : null,
      }),
    onSuccess: () => {
      setWalletLabel("");
      setWalletAddress("");
      refresh();
    },
  });

  const recordBalance = useMutation({
    mutationFn: () =>
      recordCryptoBalance(balanceWallet, {
        as_of: balanceDate,
        quantity: balanceQuantity,
      }),
    onSuccess: () => {
      setBalanceQuantity("");
      refresh();
    },
  });

  const syncWallet = useMutation({
    mutationFn: (walletId: string) => syncCryptoWallet(walletId),
    onSuccess: refresh,
  });

  const savePrice = useMutation({
    mutationFn: () => setPrice({ symbol: priceSymbol, price: priceValue }),
    onSuccess: () => {
      setPriceValue("");
      refresh();
    },
  });

  const handleBulk = async (rows: GridRow[]): Promise<SaveResult> => {
    const errors: { index: number; error: string }[] = [];
    let savedCount = 0;
    for (const [index, row] of rows.entries()) {
      const asset = (assets.data ?? []).find(
        (item) => item.symbol.toUpperCase() === row.asset.toUpperCase(),
      );
      if (!asset) {
        errors.push({ index, error: `unknown asset '${row.asset}'` });
        continue;
      }
      const kind = row.kind === "address" ? "address" : "manual";
      try {
        const wallet = await createCryptoWallet({
          asset_id: asset.id,
          kind,
          chain: asset.chain,
          label: row.label,
          address: kind === "address" ? row.address : null,
        });
        if (kind === "manual" && row.quantity) {
          await recordCryptoBalance(wallet.id, {
            as_of: row.as_of || new Date().toISOString().slice(0, 10),
            quantity: row.quantity,
          });
        }
        savedCount += 1;
      } catch (error) {
        errors.push({ index, error: (error as Error).message });
      }
    }
    refresh();
    return { savedCount, errors };
  };

  const walletById = useMemo(
    () => new Map((wallets.data ?? []).map((wallet) => [wallet.id, wallet])),
    [wallets.data],
  );

  return (
    <section aria-label="crypto">
      <section className="card" aria-label="crypto positions">
        <h2>Crypto</h2>
        <p className="muted">
          Track native coins by public address or as a manual wallet/bucket.
          Address sync is optional; manual balances and prices work offline.
        </p>

        <div className="controls">
          <span className="badge">
            {provider.data?.configured
              ? `chain add-on: ${provider.data.name}`
              : "manual balances only"}
          </span>
        </div>

        {positions.isPending && <p className="muted">Loading positions …</p>}
        {positions.isError && (
          <p className="error" role="alert">
            {(positions.error as Error).message}
          </p>
        )}
        {positions.data && positions.data.length === 0 && (
          <p className="muted">No crypto positions yet.</p>
        )}
        {positions.data && positions.data.length > 0 && (
          <table className="table">
            <thead>
              <tr>
                <th>Asset</th>
                <th>Quantity</th>
                <th>Wallets</th>
                <th>Price</th>
                <th>Value</th>
              </tr>
            </thead>
            <tbody>
              {positions.data.map((position) => (
                <tr key={position.asset_id}>
                  <td>
                    {position.symbol}
                    <span className="muted"> · {position.chain}</span>
                  </td>
                  <td>{position.quantity}</td>
                  <td>{position.wallet_count}</td>
                  <td>
                    {formatAmount(position.price, position.currency)}
                    {position.price_stale && (
                      <span className="badge badge-warn"> stale</span>
                    )}
                    {!position.price && (
                      <span className="badge badge-warn"> unpriced</span>
                    )}
                  </td>
                  <td>{formatAmount(position.value, position.currency)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}

        {wallets.data && wallets.data.length > 0 && (
          <section aria-label="crypto wallets">
            <h3>Wallets</h3>
            <table className="table">
              <thead>
                <tr>
                  <th>Wallet</th>
                  <th>Asset</th>
                  <th>Kind</th>
                  <th>Address</th>
                  <th aria-label="actions" />
                </tr>
              </thead>
              <tbody>
                {wallets.data.map((wallet: CryptoWallet) => (
                  <tr key={wallet.id}>
                    <td>{wallet.label}</td>
                    <td>{wallet.symbol}</td>
                    <td>{wallet.kind}</td>
                    <td className="mono">{wallet.address ?? "—"}</td>
                    <td>
                      {wallet.kind === "address" && (
                        <button
                          type="button"
                          aria-label={`sync ${wallet.label}`}
                          onClick={() => syncWallet.mutate(wallet.id)}
                          disabled={!canWrite || !provider.data?.configured}
                        >
                          Sync
                        </button>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </section>
        )}
      </section>

      <section className="card" aria-label="add crypto asset">
        <h2>Add asset</h2>
        <form
          className="row"
          onSubmit={(event) => {
            event.preventDefault();
            addAsset.mutate();
          }}
        >
          <label>
            <span>Symbol</span>
            <input
              aria-label="crypto asset symbol"
              placeholder="BTC"
              value={assetSymbol}
              onChange={(event) => setAssetSymbol(event.target.value)}
            />
          </label>
          <label>
            <span>Name</span>
            <input
              aria-label="crypto asset name"
              placeholder="Bitcoin"
              value={assetName}
              onChange={(event) => setAssetName(event.target.value)}
            />
          </label>
          <label>
            <span>Chain</span>
            <select
              aria-label="crypto asset chain"
              value={assetChain}
              onChange={(event) =>
                setAssetChain(event.target.value as (typeof CHAINS)[number])
              }
            >
              {CHAINS.map((chain) => (
                <option key={chain} value={chain}>
                  {chain}
                </option>
              ))}
            </select>
          </label>
          <label>
            <span>Decimals</span>
            <input
              aria-label="crypto asset decimals"
              value={assetDecimals}
              onChange={(event) => setAssetDecimals(event.target.value)}
            />
          </label>
          <button type="submit" disabled={!canWrite || addAsset.isPending}>
            Add asset
          </button>
        </form>
        {addAsset.isError && (
          <p className="error" role="alert">
            {(addAsset.error as Error).message}
          </p>
        )}
      </section>

      <section className="card" aria-label="add crypto wallet">
        <h2>Add wallet</h2>
        <form
          className="row"
          onSubmit={(event) => {
            event.preventDefault();
            addWallet.mutate();
          }}
        >
          <label>
            <span>Asset</span>
            <select
              aria-label="crypto wallet asset"
              value={walletAsset}
              onChange={(event) => setWalletAsset(event.target.value)}
            >
              <option value="">Select …</option>
              {(assets.data ?? []).map((asset) => (
                <option key={asset.id} value={asset.id}>
                  {asset.symbol} — {asset.name}
                </option>
              ))}
            </select>
          </label>
          <label>
            <span>Kind</span>
            <select
              aria-label="crypto wallet kind"
              value={walletKind}
              onChange={(event) =>
                setWalletKind(event.target.value as "manual" | "address")
              }
            >
              <option value="manual">manual</option>
              <option value="address">address</option>
            </select>
          </label>
          <label>
            <span>Label</span>
            <input
              aria-label="crypto wallet label"
              placeholder="Coinbase"
              value={walletLabel}
              onChange={(event) => setWalletLabel(event.target.value)}
            />
          </label>
          {walletKind === "address" && (
            <label>
              <span>Public address</span>
              <input
                aria-label="crypto wallet address"
                value={walletAddress}
                onChange={(event) => setWalletAddress(event.target.value)}
              />
            </label>
          )}
          <button type="submit" disabled={!canWrite || addWallet.isPending}>
            Add wallet
          </button>
        </form>
        {addWallet.isError && (
          <p className="error" role="alert">
            {(addWallet.error as Error).message}
          </p>
        )}
      </section>

      <section className="card" aria-label="record crypto balance">
        <h2>Record balance</h2>
        <form
          className="row"
          onSubmit={(event) => {
            event.preventDefault();
            recordBalance.mutate();
          }}
        >
          <label>
            <span>Wallet</span>
            <select
              aria-label="crypto balance wallet"
              value={balanceWallet}
              onChange={(event) => setBalanceWallet(event.target.value)}
            >
              <option value="">Select …</option>
              {(wallets.data ?? []).map((wallet) => (
                <option key={wallet.id} value={wallet.id}>
                  {wallet.label} · {wallet.symbol}
                </option>
              ))}
            </select>
          </label>
          <label>
            <span>As of</span>
            <input
              aria-label="crypto balance date"
              placeholder="YYYY-MM-DD"
              value={balanceDate}
              onChange={(event) => setBalanceDate(event.target.value)}
            />
          </label>
          <label>
            <span>Quantity</span>
            <input
              aria-label="crypto balance quantity"
              placeholder="0.003"
              value={balanceQuantity}
              onChange={(event) => setBalanceQuantity(event.target.value)}
            />
          </label>
          <button type="submit" disabled={!canWrite || recordBalance.isPending}>
            Record balance
          </button>
        </form>
        {recordBalance.isError && (
          <p className="error" role="alert">
            {(recordBalance.error as Error).message}
          </p>
        )}
        {recordBalance.data && (
          <p role="status">
            Recorded {recordBalance.data.quantity}{" "}
            {walletById.get(recordBalance.data.wallet_id)?.symbol ?? ""}
          </p>
        )}
      </section>

      <section className="card" aria-label="crypto manual price">
        <h2>Manual price</h2>
        <form
          className="row"
          onSubmit={(event) => {
            event.preventDefault();
            savePrice.mutate();
          }}
        >
          <label>
            <span>Asset</span>
            <select
              aria-label="crypto price asset"
              value={priceSymbol}
              onChange={(event) => setPriceSymbol(event.target.value)}
            >
              <option value="">Select …</option>
              {(assets.data ?? []).map((asset) => (
                <option key={asset.id} value={asset.symbol}>
                  {asset.symbol}
                </option>
              ))}
            </select>
          </label>
          <label>
            <span>Price</span>
            <input
              aria-label="crypto manual price"
              value={priceValue}
              onChange={(event) => setPriceValue(event.target.value)}
            />
          </label>
          <button type="submit" disabled={!canWrite || savePrice.isPending}>
            Save price
          </button>
        </form>
        {savePrice.isError && (
          <p className="error" role="alert">
            {(savePrice.error as Error).message}
          </p>
        )}
      </section>

      <section className="card" aria-label="crypto bulk entry">
        <h2>Bulk wallets</h2>
        <p className="muted">
          Paste address rows or manual wallet amounts. Each row creates a wallet;
          manual rows also record the quantity.
        </p>
        <BulkGrid
          columns={BULK_COLUMNS}
          minRows={1}
          onSave={handleBulk}
          saveLabel="Save wallets"
          readOnly={!canWrite}
        />
      </section>
    </section>
  );
}
