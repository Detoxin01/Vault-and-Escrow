use anchor_lang::prelude::*;

#[error_code]
pub enum VaultError {
    #[msg("Amount must be greater than zero")]
    InvalidAmount,
    #[msg("Withdrawal amount exceeds the available vault balance")]
    InsufficientFunds,
}
