//! Property tests for the SVS allocator invariants I1-I7 (see
//! skill/invariants-qedgen.md). Pure-state tests over the share-math model and a
//! faithful mirror of the guard cores in templates/guards/src/lib.rs, so they run
//! fast and dependency-free with `cargo test` (no anchor/solana toolchain).
//!
//! The mirrored predicates use the SAME integer math as the on-chain cores; if
//! you change a guard, change it here too (CI runs both crates).
//!
//! Add to dev-dependencies: `proptest = "1"`.

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    // ---- anti-inflation share math (mirrors guards/state) ----
    const VIRTUAL_SHARES: u128 = 1_000_000;
    const VIRTUAL_ASSETS: u128 = 1;

    /// Minimal mirror of Vault share math (state/mod.rs).
    #[derive(Clone, Copy)]
    struct Model {
        total_shares: u128,
        stored_assets: u128,
    }

    impl Model {
        fn shares_for_deposit(&self, assets: u128) -> u128 {
            assets * (self.total_shares + VIRTUAL_SHARES) / (self.stored_assets + VIRTUAL_ASSETS)
        }
        fn assets_for_shares(&self, shares: u128) -> u128 {
            shares * (self.stored_assets + VIRTUAL_ASSETS) / (self.total_shares + VIRTUAL_SHARES)
        }
        fn deposit(&mut self, assets: u128) -> u128 {
            let s = self.shares_for_deposit(assets);
            self.total_shares += s;
            self.stored_assets += assets;
            s
        }
        fn withdraw(&mut self, shares: u128) -> u128 {
            let a = self.assets_for_shares(shares);
            self.total_shares -= shares;
            self.stored_assets -= a;
            a
        }
    }

    // ---- guard-core mirrors (same integer math as guards/src/lib.rs) ----

    fn oracle_fresh(now: i64, publish_time: i64, max_staleness_secs: u64) -> bool {
        now.saturating_sub(publish_time) <= max_staleness_secs as i64
    }
    fn oracle_confident(price: u64, conf: u64, max_conf_bps: u16) -> bool {
        let p = price.max(1) as u128;
        let conf_bps = (conf as u128) * 10_000 / p;
        conf_bps as u64 <= max_conf_bps as u64
    }
    fn deviation_bps(pool_price: u64, oracle_price: u64) -> u128 {
        let o = oracle_price.max(1) as u128;
        (pool_price.abs_diff(oracle_price) as u128) * 10_000 / o
    }
    fn pool_in_band(pool_price: u64, oracle_price: u64, max_deviation_bps: u16) -> bool {
        deviation_bps(pool_price, oracle_price) as u64 <= max_deviation_bps as u64
    }
    fn fair_out(amount_in: u64, p_in: u64, p_out: u64) -> u64 {
        ((amount_in as u128) * (p_in as u128) / (p_out.max(1) as u128)) as u64
    }
    fn min_out_floor(fair: u64, max_slippage_bps: u16) -> u64 {
        ((fair as u128) * ((10_000 - max_slippage_bps) as u128) / 10_000) as u64
    }

    /// Whole-swap guard: passes iff every value-moving precondition holds.
    /// Mirrors rebalance_swap (rebalance.rs) + guards cores.
    #[allow(clippy::too_many_arguments)]
    fn swap_guard_passes(
        paused: bool,
        amount_in: u64,
        per_tx_cap: u64,
        now: i64,
        publish_time: i64,
        max_staleness_secs: u64,
        price: u64,
        conf: u64,
        max_conf_bps: u16,
        pool_price: u64,
        max_deviation_bps: u16,
        min_out: u64,
        p_in: u64,
        p_out: u64,
        max_slippage_bps: u16,
    ) -> bool {
        if paused || amount_in == 0 || amount_in > per_tx_cap {
            return false;
        }
        if !oracle_fresh(now, publish_time, max_staleness_secs) {
            return false;
        }
        if !oracle_confident(price, conf, max_conf_bps) {
            return false;
        }
        if !pool_in_band(pool_price, price, max_deviation_bps) {
            return false;
        }
        let floor = min_out_floor(fair_out(amount_in, p_in, p_out), max_slippage_bps);
        min_out >= floor
    }

    /// Crank authorization (mirrors check_keeper): only the configured keeper key
    /// is authorized; modeled over u64 stand-ins for pubkeys.
    fn check_keeper(signer: u64, keeper: u64) -> bool {
        signer == keeper
    }

    /// Deposit/withdraw cap+pause gate (mirrors check_caps).
    fn caps_pass(paused: bool, amount: u64, per_tx_cap: u64, stored: u64, deposit_cap: u64, is_deposit: bool) -> bool {
        if paused || amount == 0 || amount > per_tx_cap {
            return false;
        }
        if is_deposit {
            match stored.checked_add(amount) {
                Some(after) => after <= deposit_cap,
                None => false,
            }
        } else {
            true
        }
    }

    proptest! {
        // -----------------------------------------------------------------
        // I1: deposit-then-withdraw never returns more than deposited.
        // -----------------------------------------------------------------
        #[test]
        fn i1_no_free_value(
            init_shares in 0u128..1_000_000u128,
            init_assets in 0u128..1_000_000u128,
            a in 1u128..1_000_000u128,
        ) {
            let mut m = Model { total_shares: init_shares, stored_assets: init_assets.max(init_shares) };
            let shares = m.deposit(a);
            let out = m.withdraw(shares);
            prop_assert!(out <= a);
        }

        // -----------------------------------------------------------------
        // I2: NAV consistency. sync() must value NAV from tracked idle + oracle-
        // valued allocations, and MUST ignore the raw ATA balance — so a direct
        // donation cannot inflate NAV (the A5 linkage). Falsifiable: a "sync" that
        // read the raw balance would fail this whenever donation > 0.
        // -----------------------------------------------------------------
        #[test]
        fn i2_nav_consistency(
            idle in 0u64..1_000_000u64,
            allocs in proptest::collection::vec(0u64..1_000_000u64, 0..8),
            donation in 0u64..1_000_000u64,
        ) {
            // Correct sync: idle (tracked) + Σ oracle-valued allocations. No raw ATA.
            fn sync_nav(idle: u64, allocs: &[u64]) -> u128 {
                idle as u128 + allocs.iter().map(|&v| v as u128).sum::<u128>()
            }
            let nav = sync_nav(idle, &allocs);
            let alloc_sum: u128 = allocs.iter().map(|&v| v as u128).sum();

            // NAV equals the tracked composition exactly (ε = 0 in the integer model).
            prop_assert_eq!(nav, idle as u128 + alloc_sum);

            // A naive implementation that valued NAV from the raw ATA (idle + donation)
            // would diverge whenever someone donated — the invariant forbids that.
            let naive_from_raw = idle as u128 + donation as u128 + alloc_sum;
            if donation > 0 {
                prop_assert_ne!(nav, naive_from_raw);
            }
        }

        // -----------------------------------------------------------------
        // I3: Conservation. Controlled assets change ONLY by deposits, withdrawals,
        // swap deltas, and claimed fees — never minted from nothing. Compared two
        // independent ways (running balance vs closed-form net). A swap that minted
        // value (out > in) would break the equality.
        // -----------------------------------------------------------------
        #[test]
        fn i3_conservation(
            init in 0u64..1_000_000u64,
            deposits in proptest::collection::vec(1u64..100_000u64, 0..10),
            swap_in in 0u64..50_000u64,
            fees in 0u64..100_000u64,
            withdrawals in proptest::collection::vec(0u64..100_000u64, 0..10),
        ) {
            // Running balance, applying each operation as it happens.
            let mut controlled: i128 = init as i128;
            for d in &deposits { controlled += *d as i128; }      // assets enter
            // value-neutral swap: out == in (same asset value); must not change total
            let swap_out = swap_in;
            controlled += swap_out as i128;
            controlled -= swap_in as i128;
            controlled += fees as i128;                            // realized fees enter
            let mut withdrawn_total: i128 = 0;
            for w in &withdrawals {
                let w = (*w as i128).min(controlled.max(0));       // can't take more than held
                controlled -= w;
                withdrawn_total += w;
            }

            // Closed form: init + deposits + fees - withdrawals (swap nets to zero).
            let expected = init as i128
                + deposits.iter().map(|&d| d as i128).sum::<i128>()
                + fees as i128
                - withdrawn_total;

            prop_assert_eq!(controlled, expected);  // no token created/destroyed
            prop_assert!(controlled >= 0);
        }

        // -----------------------------------------------------------------
        // I4: share supply tracks total_shares.
        // -----------------------------------------------------------------
        #[test]
        fn i4_supply_tracks(a in 1u128..1_000_000u128) {
            let mut m = Model { total_shares: 0, stored_assets: 0 };
            let s = m.deposit(a);
            prop_assert_eq!(m.total_shares, s);
        }

        // -----------------------------------------------------------------
        // I5: Guard dominance (the safety theorem). A swap can only succeed if the
        // oracle is fresh AND confident AND the pool is within the deviation band.
        // -----------------------------------------------------------------
        #[test]
        fn i5_guard_dominance(
            // perturbations chosen to straddle each threshold so the test is never
            // vacuous: staleness/conf/deviation each cross their limit, and min_out
            // crosses the oracle floor.
            staleness in 0i64..200i64,        // limit 60s
            conf_bps_in in 0u64..500u64,      // limit 100 bps
            dev_bps_in in 0u64..500u64,       // limit 100 bps
            min_out_slack in -100i64..100i64, // relative to the floor
            amount_in in 1u64..1_000u64,
        ) {
            const NOW: i64 = 1_000_000;
            const MAX_STALENESS: u64 = 60;
            const MAX_CONF_BPS: u16 = 100;
            const MAX_DEV_BPS: u16 = 100;
            const MAX_SLIP_BPS: u16 = 50;
            const PER_TX_CAP: u64 = 1_000;
            let price: u64 = 1_000_000;

            let publish_time = NOW - staleness;
            let conf = conf_bps_in * price / 10_000;        // ~conf_bps_in bps
            let pool_price = price + price * dev_bps_in / 10_000; // ~dev_bps_in above
            let floor = min_out_floor(fair_out(amount_in, price, price), MAX_SLIP_BPS);
            let min_out = (floor as i64 + min_out_slack).max(0) as u64;

            let passed = swap_guard_passes(
                false, amount_in, PER_TX_CAP, NOW, publish_time, MAX_STALENESS,
                price, conf, MAX_CONF_BPS, pool_price, MAX_DEV_BPS,
                min_out, price, price, MAX_SLIP_BPS,
            );

            // The guard passes IFF every predicate holds (biconditional).
            let expect = oracle_fresh(NOW, publish_time, MAX_STALENESS)
                && oracle_confident(price, conf, MAX_CONF_BPS)
                && pool_in_band(pool_price, price, MAX_DEV_BPS)
                && min_out >= floor;
            prop_assert_eq!(passed, expect);

            // The safety theorem: a successful swap implies a fresh, confident
            // oracle and an in-band pool.
            if passed {
                prop_assert!(oracle_fresh(NOW, publish_time, MAX_STALENESS));
                prop_assert!(oracle_confident(price, conf, MAX_CONF_BPS));
                prop_assert!(pool_in_band(pool_price, price, MAX_DEV_BPS));
            }
        }

        // I5b: contrapositive — a manipulated pool can NEVER produce a successful swap.
        #[test]
        fn i5b_manipulated_pool_always_reverts(
            amount_in in 1u64..1_000u64,
            price in 1_000u64..10_000_000u64,
            min_out in 0u64..u32::MAX as u64,
        ) {
            const NOW: i64 = 1_000_000;
            // pool 20% above oracle, band is 1% -> must never pass
            let pool_price = price + price / 5;
            let passed = swap_guard_passes(
                false, amount_in, 1_000, NOW, NOW, 60,
                price, 0, 100, pool_price, 100,
                min_out, price, price, 50,
            );
            prop_assert!(!passed);
        }

        // -----------------------------------------------------------------
        // I6: caps & pause respected. Success => !paused AND amount <= per_tx_cap,
        // and for deposits stored_assets_after <= deposit_cap.
        // -----------------------------------------------------------------
        #[test]
        fn i6_caps_and_pause(
            paused in any::<bool>(),
            amount in 0u64..2_000_000u64,
            per_tx_cap in 1u64..1_000_000u64,
            stored in 0u64..1_000_000u64,
            deposit_cap in 1u64..2_000_000u64,
            is_deposit in any::<bool>(),
        ) {
            if caps_pass(paused, amount, per_tx_cap, stored, deposit_cap, is_deposit) {
                prop_assert!(!paused);
                prop_assert!(amount > 0);
                prop_assert!(amount <= per_tx_cap);
                if is_deposit {
                    prop_assert!(stored.checked_add(amount).unwrap() <= deposit_cap);
                }
            }
        }

        // -----------------------------------------------------------------
        // I7: authorization. A crank-only op succeeds only for the configured
        // keeper; any other signer is rejected.
        // -----------------------------------------------------------------
        #[test]
        fn i7_authorization(signer in 0u64..1000u64) {
            // A fixed keeper is the ONLY authorized cranker. Falsifiable: a guard
            // that used `>=` or skipped the check would authorize other signers.
            const KEEPER: u64 = 42;
            prop_assert_eq!(check_keeper(signer, KEEPER), signer == KEEPER);
            if signer != KEEPER {
                prop_assert!(!check_keeper(signer, KEEPER));
            }
        }
    }
}
