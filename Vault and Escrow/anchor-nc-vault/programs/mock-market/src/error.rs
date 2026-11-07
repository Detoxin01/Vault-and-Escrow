use anchor_lang::prelude::*;

#[error_code]
pub enum MarketError {
    #[msg("Amount must be greater than zero")]
    InvalidAmount,
    #[msg("The market does not have enough idle liquidity")]
    InsufficientLiquidity,
}
