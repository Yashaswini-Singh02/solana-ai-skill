//! Vendored copy of the reusable guard middleware (see templates/guards).
//! Kept in-program so the skeleton builds standalone. Prefer depending on the
//! `vault-guards` crate in real projects to avoid drift.

use anchor_lang::prelude::*;
use crate::errors::VaultError;
use crate::state::GuardConfig;

#[derive(Clone, Copy, Debug)]
pub struct OraclePrice {
    pub price: u64,
    pub conf: u64,
    pub publish_time: i64,
}

/// Self-contained price-feed layout: `price: u64 | conf: u64 | publish_time: i64`
/// (little-endian, 24 bytes). This keeps the vault deployable and testable with
/// no external oracle dependency. Build `--features pyth` to swap in the real
/// Pyth pull-oracle read below; the rest of the guard pipeline is unchanged.
#[cfg(not(feature = "pyth"))]
pub fn read_oracle_raw(oracle_ai: &AccountInfo) -> Result<OraclePrice> {
    let data = oracle_ai.try_borrow_data()?;
    require!(data.len() >= 24, VaultError::StaleOracle);
    let price = u64::from_le_bytes(data[0..8].try_into().unwrap());
    let conf = u64::from_le_bytes(data[8..16].try_into().unwrap());
    let publish_time = i64::from_le_bytes(data[16..24].try_into().unwrap());
    Ok(OraclePrice { price, conf, publish_time })
}

/// Real Pyth pull-oracle read (`--features pyth`). Deserializes a
/// `PriceUpdateV2` account, after asserting it is owned by the Pyth receiver
/// program so an attacker cannot substitute a fake feed. Freshness and
/// confidence are still enforced by `read_oracle` using `GuardConfig`.
///
/// NOTE: the returned `price` is the Pyth mantissa; its scale is set by the
/// feed's `exponent`. Pass `pool_price` (and the in/out feeds in `oracle_quote`)
/// in the SAME exponent so the deviation band and `min_out` math line up. For a
/// specific feed, also bind its `feed_id` (see guards.md).
#[cfg(feature = "pyth")]
pub fn read_oracle_raw(oracle_ai: &AccountInfo) -> Result<OraclePrice> {
    use anchor_lang::AccountDeserialize;
    use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

    // The feed account MUST be owned by the Pyth pull-oracle receiver program.
    require_keys_eq!(
        *oracle_ai.owner,
        pyth_solana_receiver_sdk::ID,
        VaultError::StaleOracle
    );

    let data = oracle_ai.try_borrow_data()?;
    let update = PriceUpdateV2::try_deserialize(&mut &data[..])
        .map_err(|_| error!(VaultError::StaleOracle))?;
    let m = update.price_message;
    require!(m.price > 0, VaultError::StaleOracle);

    Ok(OraclePrice {
        price: m.price.unsigned_abs(),
        conf: m.conf,
        publish_time: m.publish_time,
    })
}

pub fn read_oracle(oracle_ai: &AccountInfo, cfg: &GuardConfig, now: i64) -> Result<u64> {
    let p = read_oracle_raw(oracle_ai)?;
    require!(
        now.saturating_sub(p.publish_time) <= cfg.max_staleness_secs as i64,
        VaultError::StaleOracle
    );
    let price = p.price.max(1);
    let conf_bps = (p.conf as u128)
        .checked_mul(10_000)
        .ok_or(VaultError::MathOverflow)?
        / price as u128;
    require!(conf_bps as u16 <= cfg.max_conf_bps, VaultError::OracleUncertain);
    Ok(p.price)
}

pub fn assert_pool_price_sane(
    oracle_ai: &AccountInfo,
    pool_price: u64,
    cfg: &GuardConfig,
) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let oracle_price = read_oracle(oracle_ai, cfg, now)?.max(1);
    let diff = pool_price.abs_diff(oracle_price);
    let dev_bps = (diff as u128)
        .checked_mul(10_000)
        .ok_or(VaultError::MathOverflow)?
        / oracle_price as u128;
    require!(dev_bps as u16 <= cfg.max_deviation_bps, VaultError::PriceManipulated);
    Ok(())
}

pub fn oracle_quote(
    oracle_in_ai: &AccountInfo,
    oracle_out_ai: &AccountInfo,
    amount_in: u64,
    cfg: &GuardConfig,
) -> Result<u64> {
    let now = Clock::get()?.unix_timestamp;
    let p_in = read_oracle(oracle_in_ai, cfg, now)?;
    let p_out = read_oracle(oracle_out_ai, cfg, now)?.max(1);
    let fair = (amount_in as u128)
        .checked_mul(p_in as u128)
        .ok_or(VaultError::MathOverflow)?
        / p_out as u128;
    Ok(fair as u64)
}

pub fn min_out_floor(fair_out: u64, cfg: &GuardConfig) -> Result<u64> {
    let floor = (fair_out as u128)
        .checked_mul((10_000 - cfg.max_slippage_bps) as u128)
        .ok_or(VaultError::MathOverflow)?
        / 10_000;
    Ok(floor as u64)
}

pub fn assert_within_caps(
    paused: bool,
    amount: u64,
    per_tx_cap: u64,
    stored_assets: u64,
    deposit_cap: u64,
    is_deposit: bool,
) -> Result<()> {
    require!(!paused, VaultError::Paused);
    require!(amount > 0, VaultError::ZeroAmount);
    require!(amount <= per_tx_cap, VaultError::CapExceeded);
    if is_deposit {
        let after = stored_assets.checked_add(amount).ok_or(VaultError::MathOverflow)?;
        require!(after <= deposit_cap, VaultError::DepositCapReached);
    }
    Ok(())
}

pub fn assert_keeper(signer: &Pubkey, cfg: &GuardConfig) -> Result<()> {
    require_keys_eq!(*signer, cfg.keeper, VaultError::Unauthorized);
    Ok(())
}
