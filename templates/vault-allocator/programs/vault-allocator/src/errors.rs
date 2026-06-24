use anchor_lang::prelude::*;

#[error_code]
pub enum VaultError {
    #[msg("vault is paused")]
    Paused,
    #[msg("amount exceeds per-transaction cap")]
    CapExceeded,
    #[msg("deposit would exceed vault cap")]
    DepositCapReached,
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
    #[msg("unauthorized signer")]
    Unauthorized,
    #[msg("arithmetic overflow")]
    MathOverflow,
    #[msg("zero amount")]
    ZeroAmount,
    #[msg("insufficient shares")]
    InsufficientShares,
}
