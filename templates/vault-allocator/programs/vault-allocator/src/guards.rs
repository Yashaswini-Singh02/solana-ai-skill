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
/// no external oracle dependency. For production, replace the body with a Pyth
/// `PriceUpdateV2` / Switchboard On-Demand deserialization (the rest of the
/// guard pipeline is unchanged).
pub fn read_oracle_raw(oracle_ai: &AccountInfo) -> Result<OraclePrice> {
    let data = oracle_ai.try_borrow_data()?;
    require!(data.len() >= 24, VaultError::StaleOracle);
    let price = u64::from_le_bytes(data[0..8].try_into().unwrap());
    let conf = u64::from_le_bytes(data[8..16].try_into().unwrap());
    let publish_time = i64::from_le_bytes(data[16..24].try_into().unwrap());
    Ok(OraclePrice { price, conf, publish_time })
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
