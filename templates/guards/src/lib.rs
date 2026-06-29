//! Reusable anti-exploit guard middleware for Solana vaults.
//!
//! See `skill/guards.md` for the threat model. The security logic is factored
//! into **pure cores** (no `AccountInfo`/`Clock`, fully unit-testable) plus thin
//! on-chain wrappers that read the oracle account and call the cores. The
//! `attack_matrix` test module exercises every core against attacks A1–A9.
//!
//! Oracle format here is a self-contained little-endian feed
//! (`price: u64 | conf: u64 | publish_time: i64`) so a vault is deployable and
//! testable with no external oracle. For production, swap `read_oracle_raw` for
//! a Pyth `PriceUpdateV2` / Switchboard On-Demand deserialization — the cores
//! and the rest of the pipeline are unchanged.

use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct GuardConfig {
    /// Price feed account (self-contained feed, Pyth, or Switchboard).
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
    #[msg("zero amount")]
    ZeroAmount,
    #[msg("arithmetic overflow")]
    MathOverflow,
}

// ---------------------------------------------------------------------------
// Anti-inflation share math (virtual offset, OZ ERC-4626 style)
// ---------------------------------------------------------------------------

/// Virtual offsets that neutralize first-deposit / donation inflation (A5),
/// combined with valuing NAV from `stored_assets` (never the raw ATA balance).
pub const VIRTUAL_SHARES: u128 = 1_000_000;
pub const VIRTUAL_ASSETS: u128 = 1;

/// Shares minted for `assets` at current NAV. Rounds DOWN (favors the vault).
pub fn shares_for_deposit(assets: u64, total_shares: u64, stored_assets: u64) -> u64 {
    ((assets as u128) * (total_shares as u128 + VIRTUAL_SHARES)
        / (stored_assets as u128 + VIRTUAL_ASSETS)) as u64
}

/// Assets returned for `shares` at current NAV. Rounds DOWN (favors the vault).
pub fn assets_for_shares(shares: u64, total_shares: u64, stored_assets: u64) -> u64 {
    ((shares as u128) * (stored_assets as u128 + VIRTUAL_ASSETS)
        / (total_shares as u128 + VIRTUAL_SHARES)) as u64
}

// ---------------------------------------------------------------------------
// Pure guard cores (unit-testable; no AccountInfo / Clock)
// ---------------------------------------------------------------------------

/// Guard 1 core: freshness + confidence. Returns the validated price.
pub fn check_oracle(p: OraclePrice, cfg: &GuardConfig, now: i64) -> core::result::Result<u64, GuardError> {
    if now.saturating_sub(p.publish_time) > cfg.max_staleness_secs as i64 {
        return Err(GuardError::StaleOracle);
    }
    let price = p.price.max(1);
    let conf_bps = (p.conf as u128) * 10_000 / price as u128;
    if conf_bps as u64 > cfg.max_conf_bps as u64 {
        return Err(GuardError::OracleUncertain);
    }
    Ok(p.price)
}

/// |pool - oracle| / oracle in basis points.
pub fn deviation_bps(pool_price: u64, oracle_price: u64) -> u128 {
    let o = oracle_price.max(1) as u128;
    (pool_price.abs_diff(oracle_price) as u128) * 10_000 / o
}

/// Guard 2 core: pool spot must be within the oracle deviation band.
pub fn check_pool_sane(pool_price: u64, oracle_price: u64, cfg: &GuardConfig) -> core::result::Result<(), GuardError> {
    if deviation_bps(pool_price, oracle_price) as u64 > cfg.max_deviation_bps as u64 {
        return Err(GuardError::PriceManipulated);
    }
    Ok(())
}

/// Oracle-fair output for `amount_in` given in/out prices.
pub fn fair_out(amount_in: u64, p_in: u64, p_out: u64) -> u64 {
    ((amount_in as u128) * (p_in as u128) / (p_out.max(1) as u128)) as u64
}

/// Guard 3 core: oracle-derived min_out floor.
pub fn min_out_floor_bps(fair: u64, max_slippage_bps: u16) -> u64 {
    ((fair as u128) * ((10_000 - max_slippage_bps) as u128) / 10_000) as u64
}

/// Guard 3 core: assert the keeper-supplied min_out clears the oracle floor.
pub fn check_min_out(min_out: u64, fair: u64, cfg: &GuardConfig) -> core::result::Result<(), GuardError> {
    if min_out < min_out_floor_bps(fair, cfg.max_slippage_bps) {
        return Err(GuardError::SlippageTooLoose);
    }
    Ok(())
}

/// Guard 4 core: pause + per-tx/deposit caps.
pub fn check_caps(
    paused: bool,
    amount: u64,
    per_tx_cap: u64,
    stored_assets: u64,
    deposit_cap: u64,
    is_deposit: bool,
) -> core::result::Result<(), GuardError> {
    if paused {
        return Err(GuardError::Paused);
    }
    if amount == 0 {
        return Err(GuardError::ZeroAmount);
    }
    if amount > per_tx_cap {
        return Err(GuardError::CapExceeded);
    }
    if is_deposit {
        let after = stored_assets.checked_add(amount).ok_or(GuardError::MathOverflow)?;
        if after > deposit_cap {
            return Err(GuardError::DepositCapReached);
        }
    }
    Ok(())
}

/// Guard 5 core: permissioned crank.
pub fn check_keeper(signer: &Pubkey, keeper: &Pubkey) -> core::result::Result<(), GuardError> {
    if signer != keeper {
        return Err(GuardError::Unauthorized);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// On-chain wrappers (read the oracle account, then call the cores)
// ---------------------------------------------------------------------------

/// Self-contained feed layout: `price|conf|publish_time` LE (24 bytes).
///
/// For production, swap this for a real Pyth pull-oracle read. The deployable
/// program template ships exactly that behind a `pyth` feature — see
/// `templates/vault-allocator/.../guards.rs` (`#[cfg(feature = "pyth")]`) and
/// `skill/guards.md`. The pure cores above (`check_oracle`, `check_pool_sane`,
/// `min_out_floor_bps`, …) are oracle-agnostic and stay unchanged.
pub fn read_oracle_raw(oracle_ai: &AccountInfo) -> Result<OraclePrice> {
    let data = oracle_ai.try_borrow_data()?;
    require!(data.len() >= 24, GuardError::StaleOracle);
    Ok(OraclePrice {
        price: u64::from_le_bytes(data[0..8].try_into().unwrap()),
        conf: u64::from_le_bytes(data[8..16].try_into().unwrap()),
        publish_time: i64::from_le_bytes(data[16..24].try_into().unwrap()),
    })
}

pub fn read_oracle(oracle_ai: &AccountInfo, cfg: &GuardConfig, now: i64) -> Result<u64> {
    Ok(check_oracle(read_oracle_raw(oracle_ai)?, cfg, now)?)
}

pub fn assert_pool_price_sane(oracle_ai: &AccountInfo, pool_price: u64, cfg: &GuardConfig) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let oracle_price = read_oracle(oracle_ai, cfg, now)?;
    Ok(check_pool_sane(pool_price, oracle_price, cfg)?)
}

pub fn oracle_quote(oracle_in_ai: &AccountInfo, oracle_out_ai: &AccountInfo, amount_in: u64, cfg: &GuardConfig) -> Result<u64> {
    let now = Clock::get()?.unix_timestamp;
    let p_in = read_oracle(oracle_in_ai, cfg, now)?;
    let p_out = read_oracle(oracle_out_ai, cfg, now)?;
    Ok(fair_out(amount_in, p_in, p_out))
}

pub fn min_out_floor(fair: u64, cfg: &GuardConfig) -> Result<u64> {
    Ok(min_out_floor_bps(fair, cfg.max_slippage_bps))
}

pub fn assert_within_caps(paused: bool, amount: u64, per_tx_cap: u64, stored_assets: u64, deposit_cap: u64, is_deposit: bool) -> Result<()> {
    Ok(check_caps(paused, amount, per_tx_cap, stored_assets, deposit_cap, is_deposit)?)
}

pub fn assert_keeper(signer: &Pubkey, cfg: &GuardConfig) -> Result<()> {
    Ok(check_keeper(signer, &cfg.keeper)?)
}

// ---------------------------------------------------------------------------
// Attack matrix A1–A9 (run with `cargo test`)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod attack_matrix {
    use super::*;

    const NOW: i64 = 1_000_000;

    fn cfg() -> GuardConfig {
        GuardConfig {
            oracle: Pubkey::new_unique(),
            max_staleness_secs: 60,
            max_conf_bps: 100,    // 1%
            max_deviation_bps: 100, // 1%
            max_slippage_bps: 50,   // 0.5%
            keeper: Pubkey::new_unique(),
        }
    }

    fn fresh(price: u64, conf: u64) -> OraclePrice {
        OraclePrice { price, conf, publish_time: NOW }
    }

    // ---- control paths (the normal flow must succeed) ----

    #[test]
    fn control_in_band_rebalance_succeeds() {
        let c = cfg();
        // oracle = 1_000_000 (scaled $1.00), pool within 1% band
        check_pool_sane(1_005_000, 1_000_000, &c).unwrap();
        let price = check_oracle(fresh(1_000_000, 500), &c, NOW).unwrap();
        let fair = fair_out(1_000, price, price);
        check_min_out(min_out_floor_bps(fair, c.max_slippage_bps), fair, &c).unwrap();
    }

    #[test]
    fn control_normal_deposit_succeeds() {
        check_caps(false, 1_000, 1_000_000, 0, u64::MAX, true).unwrap();
        assert!(shares_for_deposit(1_000, 0, 0) > 0);
    }

    // ---- A1: oracle / pool price manipulation ----

    #[test]
    fn a1_manipulated_pool_price_reverts() {
        let c = cfg();
        // pool implies $1.20 vs oracle $1.00 => 20% off, band is 1%
        assert!(matches!(
            check_pool_sane(1_200_000, 1_000_000, &c),
            Err(GuardError::PriceManipulated)
        ));
    }

    // ---- A2: sandwich / loose min_out ----

    #[test]
    fn a2_loose_min_out_reverts() {
        let c = cfg();
        let price = 1_000_000;
        let fair = fair_out(10_000, price, price); // 10_000
        let floor = min_out_floor_bps(fair, c.max_slippage_bps); // 9_950
        assert!(matches!(check_min_out(floor - 1, fair, &c), Err(GuardError::SlippageTooLoose)));
        // and the exact floor is accepted
        check_min_out(floor, fair, &c).unwrap();
    }

    // ---- A3: stale oracle ----

    #[test]
    fn a3_stale_oracle_reverts() {
        let c = cfg();
        let stale = OraclePrice { price: 1_000_000, conf: 100, publish_time: NOW - 10_000 };
        assert!(matches!(check_oracle(stale, &c, NOW), Err(GuardError::StaleOracle)));
    }

    // ---- A4: wide confidence ----

    #[test]
    fn a4_wide_confidence_reverts() {
        let c = cfg();
        // conf 5% of price, max is 1%
        let uncertain = fresh(1_000_000, 50_000);
        assert!(matches!(check_oracle(uncertain, &c, NOW), Err(GuardError::OracleUncertain)));
    }

    // ---- A5: first-deposit / donation inflation ----

    #[test]
    fn a5_first_deposit_inflation_neutralized() {
        // attacker deposits 1 unit
        let attacker_in = 1u64;
        let attacker_shares = shares_for_deposit(attacker_in, 0, 0);
        let mut total_shares = attacker_shares;
        let mut stored = attacker_in; // NAV tracks stored_assets, NOT the ATA

        // attacker DONATES 1_000_000 directly to the vault ATA.
        // NAV uses stored_assets, so the donation does NOT change share price.
        // (stored is unchanged here on purpose.)

        // victim deposits 1_000_000
        let victim_in = 1_000_000u64;
        let victim_shares = shares_for_deposit(victim_in, total_shares, stored);
        total_shares += victim_shares;
        stored += victim_in;

        // victim must receive a fair, non-zero amount of shares
        assert!(victim_shares > 0, "victim got zero shares (inflation succeeded)");

        // attacker cannot extract the victim's deposit: redeeming all attacker
        // shares returns no more than the attacker actually contributed.
        let attacker_out = assets_for_shares(attacker_shares, total_shares, stored);
        assert!(attacker_out <= attacker_in + 1, "attacker profited from inflation");

        // victim can redeem ~their deposit back
        let victim_out = assets_for_shares(victim_shares, total_shares, stored);
        assert!(victim_out >= victim_in - victim_in / 1000, "victim was diluted");
    }

    // ---- A6: unauthorized crank ----

    #[test]
    fn a6_unauthorized_crank_reverts() {
        let c = cfg();
        let attacker = Pubkey::new_unique();
        assert!(matches!(check_keeper(&attacker, &c.keeper), Err(GuardError::Unauthorized)));
        check_keeper(&c.keeper, &c.keeper).unwrap();
    }

    // ---- A7: cap breach ----

    #[test]
    fn a7_cap_breach_reverts() {
        // per-tx cap
        assert!(matches!(
            check_caps(false, 2_000_000, 1_000_000, 0, u64::MAX, true),
            Err(GuardError::CapExceeded)
        ));
        // deposit cap
        assert!(matches!(
            check_caps(false, 600, 1_000_000, 500, 1_000, true),
            Err(GuardError::DepositCapReached)
        ));
    }

    // ---- A8: paused vault ----

    #[test]
    fn a8_paused_vault_reverts() {
        assert!(matches!(
            check_caps(true, 100, 1_000_000, 0, u64::MAX, true),
            Err(GuardError::Paused)
        ));
    }

    // ---- A9: account substitution (key mismatch) ----
    //
    // Full A9 (wrong ATA / wrong PDA) is enforced at runtime by Anchor
    // constraints (`has_one`, `seeds`, `token::authority`). This models the
    // underlying key-equality predicate those constraints reduce to.
    #[test]
    fn a9_account_substitution_key_mismatch_reverts() {
        let expected_authority = Pubkey::new_unique();
        let substituted = Pubkey::new_unique();
        assert!(matches!(check_keeper(&substituted, &expected_authority), Err(GuardError::Unauthorized)));
    }
}
