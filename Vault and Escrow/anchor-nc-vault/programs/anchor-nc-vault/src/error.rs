use anchor_lang::prelude::*;

#[error_code]
pub enum VaultError {
    #[msg("Amount must be greater than zero")]
    InvalidAmount,
    #[msg("Deposit is too small to mint any shares")]
    ZeroShares,
    #[msg("Output is below the caller's minimum (slippage)")]
    SlippageExceeded,
    #[msg("Not enough idle liquidity in the vault; use redeem_in_kind instead")]
    InsufficientLiquidity,
    #[msg("Redemption would return nothing")]
    NothingToRedeem,
    #[msg("Arithmetic overflow")]
    MathOverflow,
}
