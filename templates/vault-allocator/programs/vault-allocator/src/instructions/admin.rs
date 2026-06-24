use anchor_lang::prelude::*;
use crate::state::*;
use crate::errors::VaultError;

#[derive(Accounts)]
pub struct AdminOnly<'info> {
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [b"vault", vault.authority.as_ref(), vault.asset_mint.as_ref()],
        bump = vault.bump,
        has_one = authority @ VaultError::Unauthorized,
    )]
    pub vault: Account<'info, Vault>,
}

/// Circuit breaker. Use a multisig as the authority in production.
pub fn set_paused(ctx: Context<AdminOnly>, paused: bool) -> Result<()> {
    ctx.accounts.vault.paused = paused;
    emit!(PausedSet { vault: ctx.accounts.vault.key(), paused });
    Ok(())
}

#[event]
pub struct PausedSet {
    pub vault: Pubkey,
    pub paused: bool,
}
