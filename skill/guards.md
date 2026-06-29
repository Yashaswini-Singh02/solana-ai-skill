# Vault Guards (Anti-Exploit Middleware)

The "anti-thief" layer. These are reusable, composable checks that MUST wrap
every value-moving instruction (deposit, withdraw, rebalance, swap, claim).
A vault without these is not production-grade.

Working Rust module: `../templates/guards/`.

## Threat model (what these guards stop)

| Attack | Mechanism | Guard |
| ------ | --------- | ----- |
| Price/oracle manipulation | Flash-move a pool to trick NAV or a swap | Oracle deviation + TWAP-vs-spot |
| Sandwich / MEV | Front/back-run the rebalance swap | Oracle-derived `min_out` + per-tx cap + Jito |
| First-deposit / donation inflation | Inflate share price via direct transfer | NAV from `stored_assets`, dead shares (see svs-variant-picker.md) |
| Stale oracle | Act on an old price | Max staleness + confidence-interval check |
| Unbounded loss | One bad tx drains the vault | `per_tx_cap`, `deposit_cap`, circuit-breaker |
| Unauthorized crank | Anyone calls rebalance | Permissioned keeper allowlist |

## GuardConfig (store on the Vault)

```rust
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct GuardConfig {
    pub oracle: Pubkey,             // Pyth price feed (or Switchboard)
    pub max_staleness_secs: u64,    // reject prices older than this (e.g. 60)
    pub max_conf_bps: u16,          // reject if conf/price > this (e.g. 100 = 1%)
    pub max_deviation_bps: u16,     // pool spot vs oracle band (e.g. 100 = 1%)
    pub max_slippage_bps: u16,      // swap min_out floor (e.g. 50 = 0.5%)
    pub keeper: Pubkey,            // allowed crank signer (or an allowlist PDA)
}
```

## Guard 1: oracle freshness + confidence

```rust
pub fn read_oracle(ai: &AccountInfo, cfg: &GuardConfig, now: i64) -> Result<u64> {
    let p = read_oracle_raw(ai)?;                    // self-contained OR Pyth (below)
    require!(now.saturating_sub(p.publish_time) <= cfg.max_staleness_secs as i64,
             VaultError::StaleOracle);
    let conf_bps = (p.conf as u128) * 10_000 / p.price.max(1) as u128;
    require!(conf_bps as u16 <= cfg.max_conf_bps, VaultError::OracleUncertain);
    Ok(p.price)
}
```

### Real Pyth pull-oracle read (production)

The templates ship a self-contained 24-byte feed by default so the vault builds
with no external oracle. Flip on the real Pyth read with the `pyth` feature
(`pyth-solana-receiver-sdk` 1.0.1, which pins `anchor-lang ^0.31.1`):

```bash
anchor build -- --features pyth
```

```rust
// templates/vault-allocator/.../guards.rs  (#[cfg(feature = "pyth")])
use anchor_lang::AccountDeserialize;
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

pub fn read_oracle_raw(oracle_ai: &AccountInfo) -> Result<OraclePrice> {
    // Owner MUST be the Pyth receiver program, or an attacker substitutes a feed.
    require_keys_eq!(*oracle_ai.owner, pyth_solana_receiver_sdk::ID, VaultError::StaleOracle);
    let data = oracle_ai.try_borrow_data()?;
    let update = PriceUpdateV2::try_deserialize(&mut &data[..])
        .map_err(|_| error!(VaultError::StaleOracle))?;
    let m = update.price_message;                    // price (i64), conf (u64), publish_time
    require!(m.price > 0, VaultError::StaleOracle);
    Ok(OraclePrice { price: m.price.unsigned_abs(), conf: m.conf, publish_time: m.publish_time })
}
```

Exponent: the Pyth `price` is a mantissa scaled by the feed `exponent`. Pass
`pool_price` and the in/out feeds (in `oracle_quote`) at the **same** exponent so
the deviation band and `min_out` floor compare like-for-like. Bind the feed's
`feed_id` (`get_feed_id_from_hex`) when you need a specific market.

## Guard 2: pool spot vs oracle deviation band

This is the core "anti-thief" check before any swap or LP move.

```rust
pub fn assert_pool_price_sane(oracle_ai: &AccountInfo, pool_price: u64, cfg: &GuardConfig) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let oracle_price = read_oracle(oracle_ai, cfg, now)?;
    let diff = pool_price.abs_diff(oracle_price);
    let dev_bps = (diff as u128 * 10_000 / oracle_price as u128) as u16;
    require!(dev_bps <= cfg.max_deviation_bps, VaultError::PriceManipulated);
    Ok(())
}
```

If the pool price is outside the band, the instruction reverts and (optionally)
the vault auto-pauses. A flash-loan attacker who skews the pool simply gets a
revert.

## Guard 3: oracle-derived min_out (swap floor)

See `jupiter-rebalance.md`. The program computes the fair output from the oracle
and requires `min_out >= fair_out * (1 - max_slippage_bps)`. The keeper's quote
is never the security boundary.

## Guard 4: caps + circuit breaker

```rust
require!(!vault.paused, VaultError::Paused);
require!(amount <= vault.per_tx_cap, VaultError::CapExceeded);
require!(vault.stored_assets.checked_add(amount).unwrap() <= vault.deposit_cap,
         VaultError::DepositCapReached);
```

`pause(ctx)` / `unpause(ctx)` are admin-only (or a 2-of-N multisig authority).
Consider an automatic pause when a deviation guard trips N times in a window.

## Guard 5: permissioned crank

Rebalance/swap instructions must check the signer against `vault.guard.keeper`
(or an allowlist). Deposits/withdrawals stay permissionless for users.

```rust
require_keys_eq!(ctx.accounts.cranker.key(), vault.guard.keeper, VaultError::Unauthorized);
```

## Checklist for every value-moving instruction

- [ ] `!vault.paused`
- [ ] caps (`per_tx_cap`, `deposit_cap`)
- [ ] oracle fresh + confident
- [ ] pool spot within deviation band (for LP/swap)
- [ ] oracle-derived `min_out` enforced + post-swap delta verified (for swap)
- [ ] keeper authorized (for crank-only ops)
- [ ] checked math everywhere (no silent overflow)
- [ ] event emitted for the audit trail

These map 1:1 to the formal invariants in `invariants-qedgen.md`.
