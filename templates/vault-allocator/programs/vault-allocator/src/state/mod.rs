use anchor_lang::prelude::*;

pub const MAX_VENUES: usize = 8;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct GuardConfig {
    pub oracle: Pubkey,
    pub max_staleness_secs: u64,
    pub max_conf_bps: u16,
    pub max_deviation_bps: u16,
    pub max_slippage_bps: u16,
    pub keeper: Pubkey,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, Default)]
pub struct Allocation {
    pub venue_id: u8,        // 0 = idle, 1 = meteora, 2 = orca, ...
    pub deployed: u64,       // raw asset units committed to this venue
    pub last_nav: u64,       // last oracle-valued NAV of this allocation
}

#[account]
pub struct Vault {
    pub authority: Pubkey,
    pub asset_mint: Pubkey,
    pub share_mint: Pubkey,
    pub total_shares: u64,
    pub stored_assets: u64,          // NAV, refreshed by sync()
    pub deposit_cap: u64,
    pub per_tx_cap: u64,
    pub paused: bool,
    pub guard: GuardConfig,
    pub allocations: [Allocation; MAX_VENUES],
    pub bump: u8,
    pub auth_bump: u8,
}

impl Vault {
    // discriminator + fields; recompute if you change the struct.
    pub const SIZE: usize = 8
        + 32 * 3
        + 8 * 4
        + 1
        + (32 + 8 + 2 + 2 + 2 + 32)        // GuardConfig
        + (1 + 8 + 8) * MAX_VENUES         // allocations
        + 1 + 1;

    /// ERC-4626-style: shares minted for `assets` at current NAV.
    /// Rounds DOWN (favors the vault). First deposit is 1:1.
    pub fn shares_for_deposit(&self, assets: u64) -> Option<u64> {
        if self.total_shares == 0 || self.stored_assets == 0 {
            Some(assets)
        } else {
            Some(((assets as u128) * (self.total_shares as u128) / (self.stored_assets as u128)) as u64)
        }
    }

    /// Assets returned for `shares` at current NAV. Rounds DOWN (favors vault).
    pub fn assets_for_shares(&self, shares: u64) -> Option<u64> {
        if self.total_shares == 0 {
            Some(0)
        } else {
            Some(((shares as u128) * (self.stored_assets as u128) / (self.total_shares as u128)) as u64)
        }
    }
}
