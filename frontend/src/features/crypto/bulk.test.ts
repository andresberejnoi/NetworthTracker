import { describe, expect, it } from "vitest";
import { groupWalletRows, walletGroupKey } from "./bulk";

describe("crypto bulk wallet grouping", () => {
  it("groups multiple rows for the same manual wallet", () => {
    const rows = [
      { asset: "BTC", label: "Coinbase", kind: "manual", as_of: "2026-01-01", quantity: "0.1" },
      { asset: "BTC", label: "coinbase", kind: "manual", as_of: "2026-02-01", quantity: "0.2" },
    ];
    const groups = groupWalletRows(rows);
    expect(groups).toHaveLength(1);
    expect(groups[0].entries).toHaveLength(2);
  });

  it("groups address rows by address and separates assets", () => {
    const address = "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq";
    expect(
      walletGroupKey({
        asset: "BTC",
        label: "Cold",
        kind: "address",
        address,
      }),
    ).toBe(walletGroupKey({ asset: "btc", label: "Other", kind: "address", address }));
    expect(
      walletGroupKey({ asset: "BTC", label: "Cold", kind: "address", address }),
    ).not.toBe(
      walletGroupKey({ asset: "ETH", label: "Cold", kind: "address", address }),
    );
  });

  it("keeps different manual labels separate", () => {
    const groups = groupWalletRows([
      { asset: "BTC", label: "Coinbase", kind: "manual" },
      { asset: "BTC", label: "Ledger", kind: "manual" },
    ]);
    expect(groups).toHaveLength(2);
  });
});
