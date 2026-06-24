# SVS Variant Picker

Goal: choose the correct Solana Vault Standard (SVS) variant for the user's
use case, then scaffold its account model. Do this before writing any CPI code.

Reference: <https://github.com/solanabr/solana-vault-standard> (sRFC 40).
SDK: `npm install @stbr/solana-vault`.

## Decision tree

Ask (or infer) two questions:

1. **Do assets leave the vault ATA?** (deployed to Meteora/Orca, lent out, bridged)
2. **Single asset or multiple assets / sub-strategies?**

```text
Assets stay in vault ATA?
├── YES, single asset, simple deposit/withdraw ........... SVS-1 (Public Live)
├── YES, native SOL .................................... SVS-7 (SOL Vault)
└── NO, assets are deployed elsewhere
    ├── single strategy, authority reports yield ........ SVS-2 (Public Stored, needs sync())
    ├── multiple assets, oracle-weighted basket ......... SVS-8 (Multi-Asset Basket)  <-- common
    ├── allocates across child vaults / venues via CPI .. SVS-9 (Allocator)           <-- common
    └── deposits/withdrawals are queued (request->fulfill) SVS-10 (Async, ERC-7540 style)
Need privacy / confidential balances? ................... SVS-3/4 (+ Token-2022 confidential)
Need streaming yield accrual? ........................... SVS-5/6 (Streaming)
```

### Why SVS-8 and SVS-9 are the focus of this skill

The canonical "automated investment vault that deploys funds into Meteora and
Orca" maps to:

- **SVS-9 Allocator** — the vault holds a *stored balance* and CPIs into child
  venues (Meteora DLMM position, Orca Whirlpool position). The keeper triggers
  reallocation. Best when you actively move funds between venues.
- **SVS-8 Multi-Asset Basket** — oracle-weighted holdings with weight
  rebalancing. Best when the vault holds a target-weight portfolio and
  rebalances ratios.

If the vault both holds a basket AND deploys to LP venues, use **SVS-9** as the
outer shell and treat each venue position as a child allocation.

## Required account model (SVS-9 Allocator)

Use canonical PDAs. Seeds below are the convention; keep them stable.

| Account            | Seeds                                  | Purpose |
| ------------------ | -------------------------------------- | ------- |
| `Vault` (config)   | `[b"vault", authority, asset_mint]`    | Config, caps, paused flag, total shares, allocations |
| `vault_authority`  | `[b"vault_auth", vault]`               | PDA signer for all CPIs and ATAs |
| `share_mint`       | `[b"shares", vault]`                   | SPL mint representing vault shares |
| `vault_ata`        | ATA(`vault_authority`, `asset_mint`)   | Idle reserve held by the vault |
| `Allocation[i]`    | `[b"alloc", vault, venue_id]`          | Per-venue accounting (deployed amount, last NAV) |

### `Vault` state (minimum fields)

```rust
#[account]
pub struct Vault {
    pub authority: Pubkey,        // admin / manager
    pub asset_mint: Pubkey,       // the deposit asset (e.g. USDC)
    pub share_mint: Pubkey,
    pub total_shares: u64,        // outstanding shares
    pub stored_assets: u64,       // last-synced NAV across idle + allocations
    pub deposit_cap: u64,         // hard cap; reject deposits beyond this
    pub per_tx_cap: u64,          // max single deposit/withdraw
    pub paused: bool,             // circuit breaker
    pub guard: GuardConfig,       // oracle + slippage params (see guards.md)
    pub bump: u8,
    pub auth_bump: u8,
}
```

## Share accounting (do not get this wrong)

SVS uses ERC-4626-style share math. On deposit/withdraw, compute against the
current NAV (`stored_assets`), never against the raw ATA balance (which can be
inflated via direct transfer — a classic first-deposit / donation attack).

```text
shares_out = assets_in * total_shares / stored_assets        (total_shares > 0)
shares_out = assets_in                                        (first deposit, 1:1)
assets_out = shares_in * stored_assets / total_shares
```

Mitigations (enforce these):
- Seed an initial "dead shares" deposit (mint a small number of shares to a
  burn address on init) to neutralize first-deposit inflation.
- Use `stored_assets` from `sync()` / NAV calc, never `vault_ata.amount`.
- Round shares *down* on deposit and assets *down* on withdraw (favor the vault).

See `svs-interface.md` for the full instruction signatures and `preview_*`
view functions.

## Scaffold checklist

1. `cargo new`/`anchor init` or copy `../templates/vault-allocator/`.
2. Define `Vault`, `Allocation`, `GuardConfig` (state).
3. Implement `initialize`, `deposit`, `withdraw`, `sync` (SVS interface).
4. Add venue CPIs: `meteora-dlmm-cpi.md`, `orca-whirlpool-cpi.md`.
5. Add rebalancing: `jupiter-rebalance.md`.
6. Wire guards into every value-moving instruction: `guards.md`.
7. Add invariants + attack tests: `invariants-qedgen.md`, `attack-tests.md`.
8. Build the keeper: `keeper-crank.md`.
