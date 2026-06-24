use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenInterface, TokenAccount, transfer_checked, TransferChecked, mint_to, MintTo};
use crate::state::*;
use crate::errors::VaultError;
use crate::guards;

#[derive(Accounts)]
pub struct Deposit<'info> {
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

pub fn deposit(ctx: Context<Deposit>, assets: u64) -> Result<()> {
    let vault = &ctx.accounts.vault;

    // Guard 4: pause + caps. NAV (stored_assets) must be fresh via sync().
    guards::assert_within_caps(
        vault.paused,
        assets,
        vault.per_tx_cap,
        vault.stored_assets,
        vault.deposit_cap,
        true,
    )?;

    // Compute shares against NAV, never against raw ATA balance (anti-inflation).
    let shares = vault.shares_for_deposit(assets).ok_or(VaultError::MathOverflow)?;
    require!(shares > 0, VaultError::ZeroAmount);

    // Pull assets from the user.
    transfer_checked(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.user_asset_ata.to_account_info(),
                mint: ctx.accounts.asset_mint.to_account_info(),
                to: ctx.accounts.vault_ata.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        assets,
        ctx.accounts.asset_mint.decimals,
    )?;

    // Mint shares to the user (vault_authority is the mint authority).
    let vault_key = ctx.accounts.vault.key();
    let seeds: &[&[u8]] = &[b"vault_auth", vault_key.as_ref(), &[ctx.accounts.vault.auth_bump]];
    mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.share_mint.to_account_info(),
                to: ctx.accounts.user_share_ata.to_account_info(),
                authority: ctx.accounts.vault_authority.to_account_info(),
            },
            &[seeds],
        ),
        shares,
    )?;

    let vault = &mut ctx.accounts.vault;
    vault.total_shares = vault.total_shares.checked_add(shares).ok_or(VaultError::MathOverflow)?;
    vault.stored_assets = vault.stored_assets.checked_add(assets).ok_or(VaultError::MathOverflow)?;
    // idle allocation [0] tracks the vault ATA reserve
    vault.allocations[0].deployed = vault.allocations[0].deployed.checked_add(assets).ok_or(VaultError::MathOverflow)?;

    emit!(Deposited { vault: vault.key(), user: ctx.accounts.user.key(), assets, shares });
    Ok(())
}

#[event]
pub struct Deposited {
    pub vault: Pubkey,
    pub user: Pubkey,
    pub assets: u64,
    pub shares: u64,
}
