pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("BwJB4EebGWmtBBJiS1PQit6Hn1ymxeD7WPsdmrScZuzX");

/// Non-custodial vault.
///
/// * Users deposit base tokens and receive transferable share tokens.
/// * The manager can only shuffle funds between the vault's idle account and
///   one whitelisted market — never to an outside address.
/// * Users can always exit: `withdraw` (liquid, base tokens at NAV) when
///   there is idle liquidity, or `redeem_in_kind` (pro-rata slice of every
///   vault asset, incl. the market position) which works with zero liquidity.
#[program]
pub mod anchor_nc_vault {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.initialize(&ctx.bumps)
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64, min_shares: u64) -> Result<()> {
        ctx.accounts.deposit(amount, min_shares)
    }

    pub fn withdraw(ctx: Context<Withdraw>, shares: u64, min_out: u64) -> Result<()> {
        ctx.accounts.withdraw(shares, min_out)
    }

    pub fn redeem_in_kind(ctx: Context<RedeemInKind>, shares: u64) -> Result<()> {
        ctx.accounts.redeem_in_kind(shares)
    }

    pub fn allocate(ctx: Context<Rebalance>, amount: u64) -> Result<()> {
        ctx.accounts.allocate(amount)
    }

    pub fn deallocate(ctx: Context<Rebalance>, amount: u64) -> Result<()> {
        ctx.accounts.deallocate(amount)
    }
}
