# Attack-Scenario Tests

Prove the guards work by attacking the vault in tests. Use LiteSVM (or
`anchor-bankrun`) for fast, deterministic, in-process execution. Each test
asserts that the exploit **fails** (the guard reverts), and a control test
asserts the normal path **succeeds**.

Working stubs: `../templates/tests/`.

## Why LiteSVM

- Milliseconds per test, no validator process.
- Can warp time (test oracle staleness) and set arbitrary account state (set a
  manipulated pool price, a stale Pyth update, a hostile token account).

## The required attack matrix

| # | Attack | Setup | Expected |
| - | ------ | ----- | -------- |
| A1 | Oracle/price manipulation | Set pool spot far from oracle | `rebalance/swap` reverts `PriceManipulated` |
| A2 | Sandwich on rebalance | Provide `min_out` below oracle floor | reverts `SlippageTooLoose`; post-swap delta check reverts `SlippageExceeded` |
| A3 | Stale oracle | Warp clock past `max_staleness_secs` | reverts `StaleOracle` |
| A4 | Wide confidence | Set oracle conf > `max_conf_bps` | reverts `OracleUncertain` |
| A5 | First-deposit inflation | Attacker deposits 1, donates large amount to ATA, victim deposits | victim shares ≈ fair; attacker cannot steal (dead shares + NAV from stored_assets) |
| A6 | Unauthorized crank | Non-keeper signer calls rebalance | reverts `Unauthorized` |
| A7 | Cap breach | Deposit > `per_tx_cap` or NAV > `deposit_cap` | reverts `CapExceeded` / `DepositCapReached` |
| A8 | Paused vault | Pause, then any value move | reverts `Paused` |
| A9 | Account substitution | Pass attacker ATA / wrong PDA | reverts (Anchor constraint / `has_one`) |

## Example (LiteSVM, TypeScript)

```ts
import { LiteSVM } from "litesvm";
import { test, expect } from "vitest";

test("A1: manipulated pool price reverts rebalance", () => {
  const svm = new LiteSVM();
  const env = setupVault(svm); // deploys program, inits vault + guard config

  // Set Pyth feed to $1.00, but skew the pool to imply $1.20 (20% off, band=1%)
  env.setOraclePrice(1.00);
  env.setPoolPrice(1.20);

  const ix = env.buildRebalanceIx({ minOut: env.oracleFloor() });
  const res = svm.sendTransaction(env.txWithKeeper(ix));

  expect(res.toString()).toContain("PriceManipulated"); // guard tripped
});

test("control: in-band rebalance succeeds", () => {
  const svm = new LiteSVM();
  const env = setupVault(svm);
  env.setOraclePrice(1.00);
  env.setPoolPrice(1.005); // within 1% band
  const ix = env.buildRebalanceIx({ minOut: env.oracleFloor() });
  const res = svm.sendTransaction(env.txWithKeeper(ix));
  expect(res.err).toBeUndefined();
});
```

## Example (Rust property test for I1)

See `invariants-qedgen.md` for the full set; run with `cargo test`.

## Coverage gate

Do not mark a vault production-ready until A1–A9 all pass plus the invariant
property tests I1–I7. This is referenced by `trail-of-bits-checklist.md` section 8.
