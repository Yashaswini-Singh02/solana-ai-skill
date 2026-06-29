use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenInterface, TokenAccount, transfer_checked, TransferChecked, burn, Burn};
use crate::state::*;
use crate::errors::VaultError;
use crate::guards;

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"vault", vault.authority.as_ref(), vault.asset_mint.as_ref()],
        bump = vault.bump,
        has_one = asset_mint,
        has_one = share_mint,
    )]
    pub vault: Account<'info, Vault>,

    /// CHECK: PDA signer; validated by seeds.
    #[account(seeds = [b"vault_auth", vault.key().as_ref()], bump = vault.auth_bump)]
    pub vault_authority: UncheckedAccount<'info>,

    pub asset_mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub share_mint: InterfaceAccount<'info, Mint>,

    #[account(mut, token::mint = asset_mint, token::authority = user)]
    pub user_asset_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(mut, token::mint = asset_mint, token::authority = vault_authority)]
    pub vault_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(mut, token::mint = share_mint, token::authority = user)]
    pub user_share_ata: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub fn withdraw(ctx: Context<Withdraw>, shares: u64) -> Result<()> {
    let vault = &ctx.accounts.vault;
    require!(!vault.paused, VaultError::Paused);
    require!(shares > 0, VaultError::ZeroAmount);
    require!(shares <= vault.total_shares, VaultError::InsufficientShares);

    // Assets owed at current NAV. Caller should ensure sync() ran recently.
    let assets = vault.assets_for_shares(shares).ok_or(VaultError::MathOverflow)?;
    require!(assets > 0, VaultError::ZeroAmount);

    // Cap consistency: bound the per-transaction withdrawal like every other
    // value-moving instruction (guards.md Guard 4 / A7).
    require!(assets <= vault.per_tx_cap, VaultError::CapExceeded);

    // NOTE: if idle reserve < assets, the keeper must first withdraw from venues
    // (Meteora/Orca) so the vault ATA holds enough. A production impl exposes a
    // queued/async path (SVS-10) for large withdrawals. Here we require liquidity.
    require!(ctx.accounts.vault_ata.amount >= assets, VaultError::InsufficientShares);

    // Burn shares first (state-before-transfer ordering).
    burn(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Burn {
                mint: ctx.accounts.share_mint.to_account_info(),
                from: ctx.accounts.user_share_ata.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        shares,
    )?;

    // Transfer assets out, signed by the vault authority PDA.
    let vault_key = ctx.accounts.vault.key();
    let seeds: &[&[u8]] = &[b"vault_auth", vault_key.as_ref(), &[ctx.accounts.vault.auth_bump]];
    transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.vault_ata.to_account_info(),
                mint: ctx.accounts.asset_mint.to_account_info(),
                to: ctx.accounts.user_asset_ata.to_account_info(),
                authority: ctx.accounts.vault_authority.to_account_info(),
            },
            &[seeds],
        ),
        assets,
        ctx.accounts.asset_mint.decimals,
    )?;

    let vault = &mut ctx.accounts.vault;
    vault.total_shares = vault.total_shares.checked_sub(shares).ok_or(VaultError::MathOverflow)?;
    vault.stored_assets = vault.stored_assets.checked_sub(assets).ok_or(VaultError::MathOverflow)?;
    vault.allocations[0].deployed = vault.allocations[0].deployed.saturating_sub(assets);

    emit!(Withdrawn { vault: vault.key(), user: ctx.accounts.user.key(), assets, shares });
    Ok(())
}

#[event]
pub struct Withdrawn {
    pub vault: Pubkey,
    pub user: Pubkey,
    pub assets: u64,
    pub shares: u64,
}
