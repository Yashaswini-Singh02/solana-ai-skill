//! Reusable anti-exploit guard middleware for Solana vaults.
//!
//! See `skill/guards.md` for the threat model. These functions are pure checks;
//! wire them into every value-moving instruction. Oracle access is abstracted
//! behind `OraclePrice` so you can plug Pyth pull oracle or Switchboard.
//!
//! NOTE: `read_oracle` is a stub — implement the deserialization for your chosen
//! oracle (Pyth `PriceUpdateV2` or Switchboard On-Demand) before use.

use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct GuardConfig {
    /// Price feed account (Pyth PriceUpdateV2 or Switchboard).
    pub oracle: Pubkey,
    /// Reject prices older than this many seconds.
    pub max_staleness_secs: u64,
    /// Reject if (conf / price) exceeds this, in basis points.
    pub max_conf_bps: u16,
    /// Max allowed |pool_spot - oracle| / oracle, in basis points.
    pub max_deviation_bps: u16,
    /// Min-out floor for swaps, in basis points of the oracle-fair output.
    pub max_slippage_bps: u16,
    /// Authorized crank signer (or an allowlist PDA).
    pub keeper: Pubkey,
}

/// Normalized oracle reading in the vault's asset terms (fixed-point, your scale).
#[derive(Clone, Copy, Debug)]
pub struct OraclePrice {
    pub price: u64,
    pub conf: u64,
    pub publish_time: i64,
}

#[error_code]
pub enum GuardError {
    #[msg("oracle price is stale")]
    StaleOracle,
    #[msg("oracle confidence interval too wide")]
    OracleUncertain,
    #[msg("pool price deviates from oracle beyond allowed band")]
    PriceManipulated,
    #[msg("provided min_out is below the oracle-derived floor")]
    SlippageTooLoose,
    #[msg("realized output below min_out")]
    SlippageExceeded,
    #[msg("vault is paused")]
    Paused,
    #[msg("amount exceeds per-transaction cap")]
    CapExceeded,
    #[msg("deposit would exceed vault cap")]
    DepositCapReached,
    #[msg("unauthorized signer")]
    Unauthorized,
    #[msg("arithmetic overflow")]
    MathOverflow,
}

/// Deserialize the oracle account into a normalized price.
/// STUB: implement for Pyth `PriceUpdateV2` / Switchboard. Kept abstract so the
/// crate stays dependency-light.
pub fn read_oracle_raw(_oracle_ai: &AccountInfo) -> Result<OraclePrice> {
    // Example for Pyth pull oracle (pseudocode):
    //   let upd = PriceUpdateV2::try_deserialize(&mut &_oracle_ai.data.borrow()[..])?;
    //   let p = upd.get_price_unchecked(); // or get_price_no_older_than
    //   Ok(OraclePrice { price: p.price as u64, conf: p.conf, publish_time: p.publish_time })
    err!(GuardError::StaleOracle) // replace with real implementation
}

/// Guard 1: freshness + confidence. Returns the validated oracle price.
pub fn read_oracle(oracle_ai: &AccountInfo, cfg: &GuardConfig, now: i64) -> Result<u64> {
    let p = read_oracle_raw(oracle_ai)?;
    require!(
        now.saturating_sub(p.publish_time) <= cfg.max_staleness_secs as i64,
        GuardError::StaleOracle
    );
    let price = p.price.max(1);
    let conf_bps = (p.conf as u128)
        .checked_mul(10_000)
        .ok_or(GuardError::MathOverflow)?
        / price as u128;
    require!(conf_bps as u16 <= cfg.max_conf_bps, GuardError::OracleUncertain);
    Ok(p.price)
}

/// Guard 2: pool spot price must be within the oracle deviation band.
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
        .ok_or(GuardError::MathOverflow)?
        / oracle_price as u128;
    require!(
        dev_bps as u16 <= cfg.max_deviation_bps,
        GuardError::PriceManipulated
    );
    Ok(())
}

/// Guard 3 helper: oracle-fair output for `amount_in` given in/out oracle prices.
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
        .ok_or(GuardError::MathOverflow)?
        / p_out as u128;
    Ok(fair as u64)
}

/// Guard 3: the oracle-derived min_out floor (call before a swap CPI).
pub fn min_out_floor(fair_out: u64, cfg: &GuardConfig) -> Result<u64> {
    let floor = (fair_out as u128)
        .checked_mul((10_000 - cfg.max_slippage_bps) as u128)
        .ok_or(GuardError::MathOverflow)?
        / 10_000;
    Ok(floor as u64)
}

/// Guard 4: caps. `paused` and `cap` checks for a value-moving instruction.
pub fn assert_within_caps(
    paused: bool,
    amount: u64,
    per_tx_cap: u64,
    stored_assets: u64,
    deposit_cap: u64,
    is_deposit: bool,
) -> Result<()> {
    require!(!paused, GuardError::Paused);
    require!(amount <= per_tx_cap, GuardError::CapExceeded);
    if is_deposit {
        let after = stored_assets
            .checked_add(amount)
            .ok_or(GuardError::MathOverflow)?;
        require!(after <= deposit_cap, GuardError::DepositCapReached);
    }
    Ok(())
}

/// Guard 5: permissioned crank.
pub fn assert_keeper(signer: &Pubkey, cfg: &GuardConfig) -> Result<()> {
    require_keys_eq!(*signer, cfg.keeper, GuardError::Unauthorized);
    Ok(())
}
