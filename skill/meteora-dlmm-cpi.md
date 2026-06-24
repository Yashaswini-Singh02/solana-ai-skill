# Meteora DLMM CPI

How a vault deposits, manages, and rebalances concentrated liquidity in a
Meteora DLMM (Dynamic Liquidity Market Maker) pool via CPI.

Docs: <https://docs.meteora.ag/developer-guide/guides/dlmm/overview>
Rust CPI examples + TS SDK are published by Meteora; pin to the version you test
against and re-verify after every upgrade (see `idl-diff-migration.md`).

## Concepts you must encode correctly

- **Bins**: DLMM splits price into discrete bins. Liquidity sits in bins around
  the active bin. `PositionV2` supports up to ~1,400 bins (was 70); byte data is
  allocated on demand as the position grows.
- **Pool types**: `Permissionless`, `PermissionlessV2` (Token-2022 support,
  created via `initialize_lb_pair2`), `Permission`, `CustomizablePermissionless`.
  Target `PermissionlessV2` for new integrations.
- **rebalance_liquidity**: a single instruction that combines add + remove +
  shift and resizes the position. Prefer it over multi-tx flows. It takes a
  `shrink_mode` (`ShrinkBoth`, `NoShrinkLeft`, `NoShrinkRight`, `NoShrinkBoth`).

## Vault integration pattern (SVS-9 child allocation)

The vault's `vault_authority` PDA owns the DLMM position. All CPIs are signed by
that PDA.

```text
deposit_to_meteora(amount):
  1. guard: assert !vault.paused
  2. guard: oracle check on pool price vs Pyth (guards.md) -> reject if deviating
  3. CPI meteora::add_liquidity_by_strategy (or rebalance_liquidity)
       signer = vault_authority PDA
       from   = vault_ata (asset) -> position bins
  4. update Allocation[meteora].deployed += amount
  5. emit event; do NOT update stored_assets here (set on sync())
```

```text
rebalance_meteora(new_lower_bin, new_upper_bin, shrink_mode):
  1. guard: assert !vault.paused
  2. guard: assert active_bin price within oracle deviation band
  3. CPI meteora::rebalance_liquidity { lower, upper, shrink_mode, ... }
  4. claim fees in the same flow (auto-compound) if configured
  5. recompute Allocation[meteora] NAV from position + unclaimed fees
```

### Required accounts (typical, verify against current IDL)

- `lb_pair`, `bin_array_bitmap_extension` (if used)
- `position` (`PositionV2`, owned by `vault_authority`)
- `user_token_x`, `user_token_y` = the vault's ATAs (owned by `vault_authority`)
- `reserve_x`, `reserve_y`, `token_x_mint`, `token_y_mint`
- `bin_array_lower`, `bin_array_upper` (passed as remaining accounts)
- `dlmm_program`, token program(s), `event_authority`

Note: if either side is a **Token-2022 mint with a transfer hook**, you must
forward the hook's extra accounts as `remaining_accounts` or the transfer fails.
Many vault skeletons strip remaining accounts on CPI token transfers — do not.

## CPI sketch (Anchor)

```rust
use anchor_lang::prelude::*;

pub fn rebalance_meteora(ctx: Context<RebalanceMeteora>, p: RebalanceParams) -> Result<()> {
    let vault = &ctx.accounts.vault;
    require!(!vault.paused, VaultError::Paused);

    // 1) Oracle deviation guard (see guards.md)
    crate::guards::assert_pool_price_sane(
        &ctx.accounts.guard_oracle,
        ctx.accounts.lb_pair.active_price()?,   // pseudo: derive from active bin
        &vault.guard,
    )?;

    // 2) PDA signer seeds for the vault authority
    let vault_key = vault.key();
    let seeds: &[&[u8]] = &[b"vault_auth", vault_key.as_ref(), &[vault.auth_bump]];
    let signer = &[seeds];

    // 3) CPI into Meteora DLMM rebalance_liquidity
    let cpi_accounts = meteora_cpi::RebalanceLiquidity {
        lb_pair: ctx.accounts.lb_pair.to_account_info(),
        position: ctx.accounts.position.to_account_info(),
        owner: ctx.accounts.vault_authority.to_account_info(),
        // ... reserves, mints, token programs, bin arrays ...
    };
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.dlmm_program.to_account_info(),
        cpi_accounts,
        signer,
    ).with_remaining_accounts(ctx.remaining_accounts.to_vec()); // bin arrays + hooks

    meteora_cpi::rebalance_liquidity(cpi_ctx, p.lower_bin, p.upper_bin, p.shrink_mode)?;

    // 4) Recompute this allocation's NAV, then let sync() roll it up.
    Ok(())
}
```

## Rebalance trigger policy (keep it cheap)

Do NOT rebalance on a fixed high-frequency timer. Use threshold triggers:

- price exits the position's bin range, OR
- realized/estimated impermanent loss exceeds a configured bps, OR
- unclaimed fees exceed a compounding threshold.

The keeper decides *when*; the program enforces *safety* (guards). See
`keeper-crank.md`.

## Gotchas

- `PositionV2` byte growth: a wider range needs a bigger position account; budget
  rent and account size.
- Re-initializing an existing bin array no longer errors in recent versions —
  don't rely on that error for control flow.
- Always claim fees before closing/shifting to avoid leaving value behind.
