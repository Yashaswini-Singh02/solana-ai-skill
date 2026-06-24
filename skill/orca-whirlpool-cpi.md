# Orca Whirlpool CPI

How a vault provides and manages concentrated liquidity (CLMM) in an Orca
Whirlpool via CPI. Orca is the second venue for the SVS-9 allocator.

Docs: <https://dev.orca.so> (Whirlpools program + SDK).

## Concepts

- **Ticks**: Whirlpools use tick-indexed concentrated liquidity. A position is
  defined by `tick_lower_index` and `tick_upper_index`, snapped to the pool's
  `tick_spacing`.
- **TickArrays**: ticks are grouped into `TickArray` accounts (88 ticks each).
  An increase/decrease/swap touches the tick arrays covering the position range;
  pass them as accounts.
- **Position**: represented by a position NFT + `Position` account. For a vault,
  the position NFT/account is owned by the `vault_authority` PDA.
- `sqrt_price` and `liquidity` use Q64.64 fixed-point math. Convert price <-> tick
  with the standard formulas; never float in on-chain math.

## Vault integration pattern (SVS-9 child allocation)

```text
deposit_to_orca(amount):
  1. guard: assert !vault.paused
  2. guard: oracle check on whirlpool price (from sqrt_price) vs Pyth
  3. CPI whirlpool::increase_liquidity (or open_position first)
       signer = vault_authority PDA
  4. Allocation[orca].deployed += amount

rebalance_orca(new_lower_tick, new_upper_tick):
  1. guard: paused + oracle band
  2. CPI decrease_liquidity (full) + collect_fees + collect_reward
  3. CPI close_position (optional) / open_position at new range
  4. CPI increase_liquidity into new range
  5. recompute Allocation[orca] NAV
```

### Required accounts (verify against current IDL)

- `whirlpool`
- `position`, `position_token_account` (NFT held by `vault_authority`)
- `token_owner_account_a/b` = vault ATAs (owned by `vault_authority`)
- `token_vault_a/b` (pool reserves)
- `tick_array_lower`, `tick_array_upper`
- `whirlpool_program`, token program(s)

## CPI sketch (Anchor)

```rust
pub fn increase_orca_liquidity(ctx: Context<IncreaseOrca>, liq: u128, max_a: u64, max_b: u64) -> Result<()> {
    let vault = &ctx.accounts.vault;
    require!(!vault.paused, VaultError::Paused);

    crate::guards::assert_pool_price_sane(
        &ctx.accounts.guard_oracle,
        crate::math::sqrt_price_to_price(ctx.accounts.whirlpool.sqrt_price)?,
        &vault.guard,
    )?;

    let vault_key = vault.key();
    let seeds: &[&[u8]] = &[b"vault_auth", vault_key.as_ref(), &[vault.auth_bump]];

    let cpi = whirlpool_cpi::IncreaseLiquidity {
        whirlpool: ctx.accounts.whirlpool.to_account_info(),
        position: ctx.accounts.position.to_account_info(),
        position_authority: ctx.accounts.vault_authority.to_account_info(),
        // token accounts, vaults, tick arrays...
    };
    let ctx_cpi = CpiContext::new_with_signer(
        ctx.accounts.whirlpool_program.to_account_info(), cpi, &[seeds]);
    whirlpool_cpi::increase_liquidity(ctx_cpi, liq, max_a, max_b)?;
    Ok(())
}
```

## Cross-venue NAV (Meteora + Orca together)

The allocator's `stored_assets` (NAV) on `sync()` =

```text
idle (vault_ata.amount)
+ value(Meteora position + unclaimed fees) in asset terms
+ value(Orca position + unclaimed fees + rewards) in asset terms
```

Value each venue position using the **oracle** price (Pyth), not the pool's own
spot price — otherwise a manipulated pool inflates NAV and lets an attacker mint
cheap shares. See `guards.md` and `invariants-qedgen.md` (NAV-consistency
invariant).

## Gotchas

- Tick indices must be multiples of `tick_spacing`; reject otherwise.
- Always `collect_fees`/`collect_reward` before `decrease_liquidity` to a closed
  position.
- Orca rewards can be a third+ mint; account for them in NAV or you'll
  under-report yield.
