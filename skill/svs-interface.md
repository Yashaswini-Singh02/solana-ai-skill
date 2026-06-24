# SVS Interface (reference — load on demand)

The Solana Vault Standard (sRFC 40) interface that compliant vaults implement.
This is the ERC-4626 equivalent adapted to Solana's account model.

Repo: <https://github.com/solanabr/solana-vault-standard>
SDK: `@stbr/solana-vault` (core, SVS-1/2). Privacy SDK `@stbr/svs-privacy-sdk`
(SVS-3/4). CLI for lifecycle ops.

## Variants (for reference)

| Variant | Name | Balance model | Sync |
| ------- | ---- | ------------- | ---- |
| SVS-1 | Public Vault (Live) | live balance | none |
| SVS-2 | Public Vault (Stored) | stored balance | `sync()` |
| SVS-3/4 | Private Vault (Live/Stored) | encrypted | per model |
| SVS-5/6 | Streaming (Public/Private) | interpolated | `distribute_yield()` + `checkpoint()` |
| SVS-7 | SOL Vault | live | native SOL wrap |
| SVS-8 | Multi-Asset Basket | oracle-weighted | weight rebalancing |
| SVS-9 | Allocator | stored | CPI to child vaults/venues |
| SVS-10 | Async (ERC-7540) | stored | request → fulfill → claim |
| SVS-11 | Credit Markets | oracle NAV | async + KYC/freeze |
| SVS-12 | Tranched | stored | manager-driven |

This skill targets **SVS-8 / SVS-9** (see `svs-variant-picker.md`).

## Core instructions (live + stored)

- `initialize(params)` — create vault config, share mint, authority PDA.
- `deposit(assets) -> shares` — pull assets, mint shares per NAV.
- `withdraw(shares) -> assets` / `redeem` — burn shares, return assets per NAV.
- `sync()` — (stored-balance vaults, SVS-2/9) recompute `stored_assets` (NAV)
  from idle + deployed allocations. Authority/keeper-gated.

## View / preview functions (must match actual results)

```text
preview_deposit(assets)  -> shares   # shares a deposit would mint now
preview_withdraw(shares) -> assets   # assets a redeem would return now
convert_to_shares(assets) -> shares
convert_to_assets(shares) -> assets
total_assets()  -> stored_assets (NAV)
total_supply()  -> total_shares
```

Compliance rule: `deposit` must mint exactly `preview_deposit(assets)`; `redeem`
must return exactly `preview_withdraw(shares)` (modulo rounding in the vault's
favor). Auditors check this.

## SDK usage (TypeScript)

```ts
import { SolanaVault, ManagedVault } from "@stbr/solana-vault";

// SVS-1 (live): assets stay in the vault ATA
const vault = new SolanaVault(connection, vaultAddress);
await vault.deposit(user, amount);

// SVS-2 / SVS-9 (stored): assets deployed elsewhere; authority syncs NAV
const managed = new ManagedVault(connection, vaultAddress);
await managed.sync(authority);   // recompute NAV before deposits/withdrawals
const shares = await managed.previewDeposit(amount);
```

Use `ManagedVault` (stored balance) for allocator/yield-aggregator vaults where
assets leave the vault ATA — which is exactly the Meteora+Orca case. Use
`SolanaVault` (live balance) only when assets never leave the vault ATA.

## Account model expectations

A compliant vault exposes a `Vault` (or `ConfidentialVault`) account whose layout
the SDK can read. Keep canonical PDA seeds (see `svs-variant-picker.md`) so
wallets/aggregators can integrate with a single integration ("build once, connect
to all vaults").

## Licensing

The standard is open-source (permissive — confirm MIT vs Apache-2.0 in the
upstream `LICENSE` before vendoring any code). This skill's own content is MIT.
Attribute SVS when you base a vault on its reference programs.
