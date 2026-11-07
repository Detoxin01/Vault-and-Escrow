use anchor_lang::prelude::*;

/// A lending-pool-like market. Suppliers deposit `base_mint` into
/// `liquidity` and receive `position_mint` 1:1 (a receipt / market position).
/// The market authority can `borrow` liquidity out, which simulates high
/// utilisation: position holders then cannot withdraw until it is repaid.
#[account]
#[derive(InitSpace)]
pub struct Market {
    pub authority: Pubkey,
    pub base_mint: Pubkey,
    pub position_mint: Pubkey,
    pub borrowed: u64,
    pub bump: u8,
    pub position_mint_bump: u8,
}
