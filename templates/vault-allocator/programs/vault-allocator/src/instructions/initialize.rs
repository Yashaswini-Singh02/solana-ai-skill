use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenInterface, TokenAccount};
use crate::state::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct InitializeParams {
    pub deposit_cap: u64,
    pub per_tx_cap: u64,
    pub guard: GuardConfig,
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    pub asset_mint: InterfaceAccount<'info, Mint>,

    #[account(
        init,
        payer = authority,
        space = Vault::SIZE,
        seeds = [b"vault", authority.key().as_ref(), asset_mint.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, Vault>,

    /// CHECK: PDA signer; validated by seeds.
    #[account(seeds = [b"vault_auth", vault.key().as_ref()], bump)]
    pub vault_authority: UncheckedAccount<'info>,

    #[account(
        init,
        payer = authority,
        seeds = [b"shares", vault.key().as_ref()],
        bump,
        mint::decimals = asset_mint.decimals,
        mint::authority = vault_authority,
    )]
    pub share_mint: InterfaceAccount<'info, Mint>,

    #[account(
        init,
        payer = authority,
        associated_token::mint = asset_mint,
        associated_token::authority = vault_authority,
    )]
    pub vault_ata: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, anchor_spl::associated_token::AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn initialize(ctx: Context<Initialize>, params: InitializeParams) -> Result<()> {
    let vault = &mut ctx.accounts.vault;
    vault.authority = ctx.accounts.authority.key();
    vault.asset_mint = ctx.accounts.asset_mint.key();
    vault.share_mint = ctx.accounts.share_mint.key();
    vault.total_shares = 0;
    vault.stored_assets = 0;
    vault.deposit_cap = params.deposit_cap;
    vault.per_tx_cap = params.per_tx_cap;
    vault.paused = false;
    vault.guard = params.guard;
    vault.allocations = Default::default();
    vault.bump = ctx.bumps.vault;
    vault.auth_bump = ctx.bumps.vault_authority;

    // TODO (anti-inflation): seed dead shares here by minting a tiny amount to a
    // burn address and recording stored_assets accordingly. See guards.md / A5.
    Ok(())
}
