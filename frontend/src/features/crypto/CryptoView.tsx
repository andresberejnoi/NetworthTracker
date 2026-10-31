import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useCurrentUser } from "../../auth/AuthContext";
import {
  createCryptoAsset,
  createCryptoLot,
  createCryptoSale,
  createCryptoWallet,
  getChainProvider,
  listCryptoAssets,
  listCryptoBalances,
  listCryptoLots,
  listCryptoPositions,
  listCryptoSales,
  listCryptoWallets,
  recordCryptoBalance,
  setPrice,
  syncCryptoWallet,
  type CryptoAsset,
  type CryptoWallet,
  type Lot,
  type Sale,
} from "../../api/client";
import { groupWalletRows } from "./bulk";
import BulkGrid, {
  type GridColumn,
  type GridRow,
  type SaveResult,
} from "../bulk-entry/BulkGrid";

const CHAINS = ["bitcoin", "ethereum"] as const;

const BULK_LOT_COLUMNS: GridColumn[] = [
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
    key: "acquired_on",
    label: "Acquired",
    placeholder: "YYYY-MM-DD",
    validate: (value) => (value === "" ? "date is required" : null),
  },
  {
    key: "quantity",
    label: "Quantity",
    validate: (value) => (value === "" ? "quantity is required" : null),
  },
  {
    key: "cost_basis",
    label: "Cost basis",
    validate: (value) => (value === "" ? "cost basis is required" : null),
  },
  {
    key: "fees",
    label: "Fees",
    placeholder: "0.00",
  },
  {
    key: "basis_quality",
    label: "Basis",
    placeholder: "actual",
  },
];

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

  const [taxWallet, setTaxWallet] = useState("");
  const [lotDate, setLotDate] = useState("");
  const [lotQuantity, setLotQuantity] = useState("");
  const [lotCost, setLotCost] = useState("");
  const [lotFees, setLotFees] = useState("");
  const [lotEstimated, setLotEstimated] = useState(false);
  const [saleDate, setSaleDate] = useState("");
  const [saleQuantity, setSaleQuantity] = useState("");
  const [saleProceeds, setSaleProceeds] = useState("");
  const [saleFees, setSaleFees] = useState("");
  const [saleYears, setSaleYears] = useState("1");

  const selectedAsset = (assets.data ?? []).find((asset) => asset.id === walletAsset);

  const refresh = () => {
    queryClient.invalidateQueries({ queryKey: ["crypto-assets"] });
    queryClient.invalidateQueries({ queryKey: ["crypto-wallets"] });
    queryClient.invalidateQueries({ queryKey: ["crypto-positions"] });
    queryClient.invalidateQueries({ queryKey: ["crypto-lots"] });
    queryClient.invalidateQueries({ queryKey: ["crypto-sales"] });
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

  const lots = useQuery({
    queryKey: ["crypto-lots", taxWallet],
    queryFn: () => listCryptoLots(taxWallet),
    enabled: Boolean(taxWallet),
  });
  const taxBalances = useQuery({
    queryKey: ["crypto-balances", taxWallet],
    queryFn: () => listCryptoBalances(taxWallet),
    enabled: Boolean(taxWallet),
  });
  const sales = useQuery({
    queryKey: ["crypto-sales", taxWallet, saleYears],
    queryFn: () =>
      listCryptoSales(taxWallet, {
        long_term_years: Number(saleYears) || 1,
      }),
    enabled: Boolean(taxWallet),
  });

  const addLot = useMutation({
    mutationFn: () =>
      createCryptoLot(taxWallet, {
        acquired_on: lotDate,
        quantity: lotQuantity,
        cost_basis: lotCost,
        fees: lotFees || null,
        basis_quality: lotEstimated ? "estimated" : "actual",
      }),
    onSuccess: () => {
      setLotQuantity("");
      setLotCost("");
      setLotFees("");
      refresh();
    },
  });

  const recordSale = useMutation({
    mutationFn: () =>
      createCryptoSale(taxWallet, {
        symbol: "",
        disposed_on: saleDate,
        quantity: saleQuantity,
        gross_proceeds: saleProceeds,
        fees: saleFees || null,
        long_term_years: Number(saleYears) || 1,
      }),
    onSuccess: () => {
      setSaleQuantity("");
      setSaleProceeds("");
      setSaleFees("");
      refresh();
    },
  });

  const savePrice = useMutation({
    mutationFn: () => setPrice({ symbol: priceSymbol, price: priceValue }),
    onSuccess: () => {
      setPriceValue("");
      refresh();
    },
  });

  const findOrCreateWallet = async (
    asset: CryptoAsset,
    kind: "manual" | "address",
    label: string,
    address: string,
  ): Promise<CryptoWallet> => {
    const existing = (wallets.data ?? []).find(
      (wallet) =>
        wallet.asset_id === asset.id &&
        wallet.kind === kind &&
        (kind === "address"
          ? wallet.address?.toLowerCase() === address.toLowerCase()
          : wallet.label.toLowerCase() === label.toLowerCase()),
    );
    if (existing) {
      return existing;
    }
    return createCryptoWallet({
      asset_id: asset.id,
      kind,
      chain: asset.chain,
      label,
      address: kind === "address" ? address : null,
    });
  };

  const handleBulk = async (rows: GridRow[]): Promise<SaveResult> => {
    const errors: { index: number; error: string }[] = [];
    let savedCount = 0;
    const today = new Date().toISOString().slice(0, 10);

    for (const group of groupWalletRows(rows)) {
      const first = group.entries[0];
      const asset = (assets.data ?? []).find(
        (item) => item.symbol.toUpperCase() === first.row.asset.toUpperCase(),
      );
      if (!asset) {
        for (const { index } of group.entries) {
          errors.push({ index, error: `unknown asset '${first.row.asset}'` });
        }
        continue;
      }
      const kind = first.row.kind === "address" ? "address" : "manual";
      const label = first.row.label?.trim() ?? "";
      const address = first.row.address?.trim() ?? "";
      let wallet: CryptoWallet;
      try {
        wallet = await findOrCreateWallet(asset, kind, label, address);
      } catch (error) {
        for (const { index } of group.entries) {
          errors.push({ index, error: (error as Error).message });
        }
        continue;
      }

      const dates = new Set<string>();
      for (const { row, index } of group.entries) {
        if (!row.quantity) {
          savedCount += 1;
          continue;
        }
        const asOf = row.as_of || today;
        if (dates.has(asOf)) {
          errors.push({ index, error: "duplicate date for this wallet" });
          continue;
        }
        dates.add(asOf);
        try {
          await recordCryptoBalance(wallet.id, {
            as_of: asOf,
            quantity: row.quantity,
          });
          savedCount += 1;
        } catch (error) {
          errors.push({ index, error: (error as Error).message });
        }
      }
    }
    refresh();
    return { savedCount, errors };
  };

  const handleLotBulk = async (rows: GridRow[]): Promise<SaveResult> => {
    const errors: { index: number; error: string }[] = [];
    let savedCount = 0;

    for (const group of groupWalletRows(rows)) {
      const first = group.entries[0];
      const asset = (assets.data ?? []).find(
        (item) => item.symbol.toUpperCase() === first.row.asset.toUpperCase(),
      );
      if (!asset) {
        for (const { index } of group.entries) {
          errors.push({ index, error: `unknown asset '${first.row.asset}'` });
        }
        continue;
      }
      const kind = first.row.kind === "address" ? "address" : "manual";
      const label = first.row.label?.trim() ?? "";
      const address = first.row.address?.trim() ?? "";
      let wallet: CryptoWallet;
      try {
        wallet = await findOrCreateWallet(asset, kind, label, address);
      } catch (error) {
        for (const { index } of group.entries) {
          errors.push({ index, error: (error as Error).message });
        }
        continue;
      }

      for (const { row, index } of group.entries) {
        try {
          await createCryptoLot(wallet.id, {
            acquired_on: row.acquired_on,
            quantity: row.quantity,
            cost_basis: row.cost_basis,
            fees: row.fees || null,
            basis_quality:
              row.basis_quality === "estimated" ? "estimated" : "actual",
          });
          savedCount += 1;
        } catch (error) {
          errors.push({ index, error: (error as Error).message });
        }
      }
    }
    refresh();
    return { savedCount, errors };
  };

  const walletById = useMemo(
    () => new Map((wallets.data ?? []).map((wallet) => [wallet.id, wallet])),
    [wallets.data],
  );
  const currency = positions.data?.[0]?.currency ?? "USD";
  const latestBalance = taxBalances.data?.[0];
  const lotQuantityTotal = useMemo(() => {
    // Display-only exact sum of remaining lot quantities at 18 decimals.
    const scale = 10n ** 18n;
    const parse = (value: string): bigint => {
      const [whole, fraction = ""] = value.split(".");
      const padded = (fraction + "0".repeat(18)).slice(0, 18);
      return BigInt(whole || "0") * scale + BigInt(padded || "0");
    };
    const total = (lots.data ?? []).reduce(
      (sum: bigint, lot: Lot) => sum + parse(lot.quantity),
      0n,
    );
    const whole = total / scale;
    const fraction = (total % scale).toString().padStart(18, "0").replace(/0+$/, "");
    return fraction ? `${whole}.${fraction}` : whole.toString();
  }, [lots.data]);
  const lotsDifferFromBalance =
    Boolean(latestBalance) &&
    (lots.data ?? []).length > 0 &&
    lotQuantityTotal !== latestBalance?.quantity.replace(/\.?0+$/, "");

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

      <section className="card" aria-label="crypto tax lots">
        <h2>Crypto tax lots</h2>
        <p className="muted">
          Purchase lots are separate from wallet balances. They drive FIFO sales
          and tax reporting; a balance can exist without lots.
        </p>
        <div className="controls">
          <label>
            <span>Wallet</span>
            <select
              aria-label="crypto tax wallet"
              value={taxWallet}
              onChange={(event) => setTaxWallet(event.target.value)}
            >
              <option value="">Select …</option>
              {(wallets.data ?? []).map((wallet) => (
                <option key={wallet.id} value={wallet.id}>
                  {wallet.label} · {wallet.symbol}
                </option>
              ))}
            </select>
          </label>
        </div>

        {taxWallet && (
          <>
            {latestBalance && (
              <p className="muted">
                Wallet balance: {latestBalance.quantity} ({latestBalance.source},{" "}
                {latestBalance.as_of})
              </p>
            )}
            {(lots.data ?? []).length > 0 && (
              <p className="muted">Tax lots remaining: {lotQuantityTotal}</p>
            )}
            {lotsDifferFromBalance && (
              <p className="badge badge-warn" role="alert">
                Tax lots total {lotQuantityTotal} differs from wallet balance{" "}
                {latestBalance?.quantity}. Add an opening lot or adjust the
                balance.
              </p>
            )}
            {latestBalance && (
              <button
                type="button"
                onClick={() => {
                  setLotDate(latestBalance.as_of);
                  setLotQuantity(latestBalance.quantity);
                }}
              >
                Use balance as opening lot
              </button>
            )}

            <h3>Lots</h3>
            {lots.data && lots.data.length === 0 && (
              <p className="muted">No purchase lots yet.</p>
            )}
            {lots.data && lots.data.length > 0 && (
              <table className="table">
                <thead>
                  <tr>
                    <th>Acquired</th>
                    <th>Remaining</th>
                    <th>Original</th>
                    <th>Basis</th>
                    <th>Source</th>
                    <th>Quality</th>
                  </tr>
                </thead>
                <tbody>
                  {lots.data.map((lot: Lot) => (
                    <tr key={lot.id}>
                      <td>{lot.acquired_on}</td>
                      <td>{lot.quantity}</td>
                      <td>{lot.original_quantity}</td>
                      <td>{formatAmount(lot.cost_basis, currency)}</td>
                      <td>{lot.source}</td>
                      <td>
                        {lot.basis_quality}
                        {lot.basis_quality === "estimated" && (
                          <span className="badge badge-warn"> estimated</span>
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}

            <h3>Add purchase lot</h3>
            <form
              className="row"
              onSubmit={(event) => {
                event.preventDefault();
                addLot.mutate();
              }}
            >
              <label>
                <span>Acquired</span>
                <input
                  aria-label="crypto lot date"
                  placeholder="YYYY-MM-DD"
                  value={lotDate}
                  onChange={(event) => setLotDate(event.target.value)}
                />
              </label>
              <label>
                <span>Quantity</span>
                <input
                  aria-label="crypto lot quantity"
                  value={lotQuantity}
                  onChange={(event) => setLotQuantity(event.target.value)}
                />
              </label>
              <label>
                <span>Cost basis</span>
                <input
                  aria-label="crypto lot cost basis"
                  value={lotCost}
                  onChange={(event) => setLotCost(event.target.value)}
                />
              </label>
              <label>
                <span>Fees</span>
                <input
                  aria-label="crypto lot fees"
                  value={lotFees}
                  onChange={(event) => setLotFees(event.target.value)}
                />
              </label>
              <label>
                <span>Estimated</span>
                <input
                  aria-label="crypto lot estimated"
                  type="checkbox"
                  checked={lotEstimated}
                  onChange={(event) => setLotEstimated(event.target.checked)}
                />
              </label>
              <button type="submit" disabled={!canWrite || addLot.isPending}>
                Add lot
              </button>
            </form>
            {addLot.isError && (
              <p className="error" role="alert">
                {(addLot.error as Error).message}
              </p>
            )}

            <h3>Record sale</h3>
            <form
              className="row"
              onSubmit={(event) => {
                event.preventDefault();
                recordSale.mutate();
              }}
            >
              <label>
                <span>Disposed</span>
                <input
                  aria-label="crypto sale date"
                  placeholder="YYYY-MM-DD"
                  value={saleDate}
                  onChange={(event) => setSaleDate(event.target.value)}
                />
              </label>
              <label>
                <span>Quantity</span>
                <input
                  aria-label="crypto sale quantity"
                  value={saleQuantity}
                  onChange={(event) => setSaleQuantity(event.target.value)}
                />
              </label>
              <label>
                <span>Proceeds</span>
                <input
                  aria-label="crypto sale proceeds"
                  value={saleProceeds}
                  onChange={(event) => setSaleProceeds(event.target.value)}
                />
              </label>
              <label>
                <span>Fees</span>
                <input
                  aria-label="crypto sale fees"
                  value={saleFees}
                  onChange={(event) => setSaleFees(event.target.value)}
                />
              </label>
              <label>
                <span>Long-term years</span>
                <input
                  aria-label="crypto sale long-term years"
                  value={saleYears}
                  onChange={(event) => setSaleYears(event.target.value)}
                />
              </label>
              <button type="submit" disabled={!canWrite || recordSale.isPending}>
                Record sale
              </button>
            </form>
            {recordSale.isError && (
              <p className="error" role="alert">
                {(recordSale.error as Error).message}
              </p>
            )}

            <h3>Realized gains</h3>
            {sales.data && sales.data.length === 0 && (
              <p className="muted">No crypto sales recorded yet.</p>
            )}
            {sales.data && sales.data.length > 0 && (
              <table className="table">
                <thead>
                  <tr>
                    <th>Disposed</th>
                    <th>Acquired</th>
                    <th>Quantity</th>
                    <th>Proceeds</th>
                    <th>Basis</th>
                    <th>Gain</th>
                    <th>Term</th>
                  </tr>
                </thead>
                <tbody>
                  {sales.data.flatMap((sale: Sale) =>
                    sale.disposals.map((disposal) => (
                      <tr key={disposal.id}>
                        <td>{sale.disposed_on}</td>
                        <td>{disposal.acquired_on}</td>
                        <td>{disposal.quantity}</td>
                        <td>{formatAmount(disposal.net_proceeds, currency)}</td>
                        <td>{formatAmount(disposal.cost_basis, currency)}</td>
                        <td>{formatAmount(disposal.realized_gain, currency)}</td>
                        <td>{disposal.term}</td>
                      </tr>
                    )),
                  )}
                </tbody>
              </table>
            )}
          </>
        )}
      </section>

      <section className="card" aria-label="crypto bulk entry">
        <h2>Bulk wallet totals</h2>
        <p className="muted">
          Rows for the same asset/label (or address) share one wallet. Each dated
          quantity is a balance observation, not a purchase transaction.
        </p>
        <BulkGrid
          columns={BULK_COLUMNS}
          minRows={1}
          onSave={handleBulk}
          saveLabel="Save totals"
          readOnly={!canWrite}
        />
      </section>

      <section className="card" aria-label="crypto bulk lots">
        <h2>Bulk purchase lots</h2>
        <p className="muted">
          Each row is a purchase lot/transaction. Multiple rows for the same
          wallet are preserved as separate lots for FIFO and tax reporting.
        </p>
        <BulkGrid
          columns={BULK_LOT_COLUMNS}
          minRows={1}
          onSave={handleLotBulk}
          saveLabel="Save lots"
          readOnly={!canWrite}
        />
      </section>
    </section>
  );
}
