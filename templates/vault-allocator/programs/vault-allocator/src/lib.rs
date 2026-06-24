//! SVS-9 Allocator Vault (skeleton).
//!
//! Implements the SVS interface (deposit / withdraw / sync) over an idle reserve
//! plus child allocations into Meteora DLMM and Orca Whirlpools, rebalanced via
//! Jupiter. Every value-moving instruction is wrapped by the guards in
//! `guards.rs`. See the matching skill docs:
//!   - skill/svs-variant-picker.md
//!   - skill/meteora-dlmm-cpi.md / skill/orca-whirlpool-cpi.md
//!   - skill/jupiter-rebalance.md
//!   - skill/guards.md
//!
//! This is an unaudited skeleton. Run skill/attack-tests.md + the checklist
//! before any deployment.

use anchor_lang::prelude::*;

pub mod errors;
pub mod guards;
pub mod state;
pub mod instructions;

use instructions::*;

declare_id!("Vau1tA11ocator1111111111111111111111111111");

#[program]
pub mod vault_allocator {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, params: InitializeParams) -> Result<()> {
        instructions::initialize(ctx, params)
    }

    /// SVS: deposit assets, mint shares at current NAV. Permissionless.
    pub fn deposit(ctx: Context<Deposit>, assets: u64) -> Result<()> {
        instructions::deposit(ctx, assets)
    }

    /// SVS: burn shares, return assets at current NAV. Permissionless.
    pub fn withdraw(ctx: Context<Withdraw>, shares: u64) -> Result<()> {
        instructions::withdraw(ctx, shares)
    }

    /// SVS (stored balance): recompute NAV from idle + allocations. Keeper-gated.
    pub fn sync(ctx: Context<Sync>) -> Result<()> {
        instructions::sync(ctx)
    }

    /// Rebalance token ratio via Jupiter. Keeper-gated + guarded.
    pub fn rebalance_swap(ctx: Context<RebalanceSwap>, amount_in: u64, min_out: u64) -> Result<()> {
        instructions::rebalance_swap(ctx, amount_in, min_out)
    }

    /// Admin circuit breaker.
    pub fn set_paused(ctx: Context<AdminOnly>, paused: bool) -> Result<()> {
        instructions::set_paused(ctx, paused)
    }
}
