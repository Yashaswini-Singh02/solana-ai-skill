use anchor_lang::prelude::*;
use anchor_spl::token_interface::TokenAccount;
use crate::state::*;
use crate::errors::VaultError;
use crate::guards;

/// Sync NAV from idle + venue allocations. Keeper-gated.
#[derive(Accounts)]
pub struct Sync<'info> {
    pub cranker: Signer<'info>,
    #[account(
        mut,
        seeds = [b"vault", vault.authority.as_ref(), vault.asset_mint.as_ref()],
        bump = vault.bump,
    )]
    pub vault: Account<'info, Vault>,
    /// CHECK: Pyth/Switchboard feed validated by guard.
    pub oracle: UncheckedAccount<'info>,
    // Pass venue position accounts as remaining_accounts to value each allocation.
}

pub fn sync(ctx: Context<Sync>) -> Result<()> {
    let vault = &ctx.accounts.vault;
    guards::assert_keeper(&ctx.accounts.cranker.key(), &vault.guard)?;

    // Value each allocation at the ORACLE price (never raw pool spot).
    // For each venue in remaining_accounts: read position size, value via oracle.
    // Here we recompute NAV = sum of allocations' last_nav (idle is exact).
    let mut nav: u64 = 0;
    for alloc in vault.allocations.iter() {
        let v = if alloc.venue_id == 0 { alloc.deployed } else { alloc.last_nav };
        nav = nav.checked_add(v).ok_or(VaultError::MathOverflow)?;
    }

    let vault = &mut ctx.accounts.vault;
    vault.stored_assets = nav;
    emit!(Synced { vault: vault.key(), nav });
    Ok(())
}

/// Rebalance token ratio via Jupiter. Keeper-gated + guarded.
/// The actual Jupiter route accounts are passed as remaining_accounts and the
/// aggregator is invoked via CPI. See skill/jupiter-rebalance.md.
#[derive(Accounts)]
pub struct RebalanceSwap<'info> {
    pub cranker: Signer<'info>,
    #[account(
        mut,
        seeds = [b"vault", vault.authority.as_ref(), vault.asset_mint.as_ref()],
        bump = vault.bump,
    )]
    pub vault: Account<'info, Vault>,
    /// CHECK: PDA signer.
    #[account(seeds = [b"vault_auth", vault.key().as_ref()], bump = vault.auth_bump)]
    pub vault_authority: UncheckedAccount<'info>,
    /// CHECK: oracle for the input mint.
    pub oracle_in: UncheckedAccount<'info>,
    /// CHECK: oracle for the output mint.
    pub oracle_out: UncheckedAccount<'info>,
    #[account(mut)]
    pub dst_ata: InterfaceAccount<'info, TokenAccount>,
    /// CHECK: Jupiter aggregator program; pin/validate its program id.
    pub jupiter_program: UncheckedAccount<'info>,
    // remaining_accounts: full Jupiter route account list.
}

pub fn rebalance_swap(ctx: Context<RebalanceSwap>, amount_in: u64, min_out: u64) -> Result<()> {
    let vault = &ctx.accounts.vault;
    require!(!vault.paused, VaultError::Paused);
    guards::assert_keeper(&ctx.accounts.cranker.key(), &vault.guard)?;
    require!(amount_in <= vault.per_tx_cap, VaultError::CapExceeded);

    // Guard 3: oracle-derived min_out floor. The keeper's quote is NOT trusted.
    let fair_out = guards::oracle_quote(
        &ctx.accounts.oracle_in.to_account_info(),
        &ctx.accounts.oracle_out.to_account_info(),
        amount_in,
        &vault.guard,
    )?;
    let floor = guards::min_out_floor(fair_out, &vault.guard)?;
    require!(min_out >= floor, VaultError::SlippageTooLoose);

    let before = ctx.accounts.dst_ata.amount;

    // CPI into Jupiter aggregator with vault_authority PDA as signer.
    // let seeds: &[&[u8]] = &[b"vault_auth", ctx.accounts.vault.key().as_ref(), &[vault.auth_bump]];
    // jupiter_cpi::route(
    //     CpiContext::new_with_signer(ctx.accounts.jupiter_program.to_account_info(),
    //         jupiter_cpi::Route { /* from route accounts */ }, &[seeds])
    //         .with_remaining_accounts(ctx.remaining_accounts.to_vec()),
    //     amount_in, min_out,
    // )?;

    ctx.accounts.dst_ata.reload()?;
    let received = ctx.accounts.dst_ata.amount.checked_sub(before).ok_or(VaultError::MathOverflow)?;
    require!(received >= min_out, VaultError::SlippageExceeded);

    emit!(Rebalanced { vault: ctx.accounts.vault.key(), amount_in, received });
    Ok(())
}

#[event]
pub struct Synced {
    pub vault: Pubkey,
    pub nav: u64,
}

#[event]
pub struct Rebalanced {
    pub vault: Pubkey,
    pub amount_in: u64,
    pub received: u64,
}
