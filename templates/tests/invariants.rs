//! Property tests for the SVS allocator invariants I1-I7 (see
//! skill/invariants-qedgen.md). Pure-state tests over the share-math model so
//! they run fast with `cargo test`. Port to your full program harness as needed.
//!
//! Add to dev-dependencies: `proptest = "1"`.

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    /// Minimal mirror of Vault share math (state/mod.rs).
    #[derive(Clone, Copy)]
    struct Model {
        total_shares: u128,
        stored_assets: u128,
    }

    // Virtual offset (anti-inflation), mirrors guards/state share math.
    const VIRTUAL_SHARES: u128 = 1_000_000;
    const VIRTUAL_ASSETS: u128 = 1;

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

    proptest! {
        // I1: deposit-then-withdraw never returns more than deposited.
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

        // I4: share supply tracks total_shares (trivially here, but guards drift).
        #[test]
        fn i4_supply_tracks(a in 1u128..1_000_000u128) {
            let mut m = Model { total_shares: 0, stored_assets: 0 };
            let s = m.deposit(a);
            prop_assert_eq!(m.total_shares, s);
        }
    }
}
