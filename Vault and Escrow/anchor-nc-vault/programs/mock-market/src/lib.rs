pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("Co9dv4kQePJoHZyLtyXnbr43sBv4hAdZgF7bFm1xeWhk");

/// A deliberately tiny lending-style market. It exists so the non-custodial
/// vault has a real "underlying market position" (the position token) and so
/// tests can simulate the market running out of liquidity (`borrow`).
#[program]
pub mod mock_market {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.initialize(&ctx.bumps)
    }

    pub fn supply(ctx: Context<Supply>, amount: u64) -> Result<()> {
        ctx.accounts.supply(amount)
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        ctx.accounts.withdraw(amount)
    }

    pub fn borrow(ctx: Context<Borrow>, amount: u64) -> Result<()> {
        ctx.accounts.borrow(amount)
    }

    pub fn repay(ctx: Context<Repay>, amount: u64) -> Result<()> {
        ctx.accounts.repay(amount)
    }
}
