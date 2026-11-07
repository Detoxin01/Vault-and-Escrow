pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("22PpE5zFUcwCxAWNNxJhiNJddUtqqBCBZMjAZBc3T7QN");

#[program]
pub mod anchor_vault {
    use super::*;

    /// Creates the user's `VaultState` PDA and funds the vault PDA with the
    /// rent-exempt minimum.
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.initialize(&ctx.bumps)
    }

    /// Moves `amount` lamports from the user into their vault.
    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        ctx.accounts.deposit(amount)
    }

    /// Moves `amount` lamports from the vault back to the user. The
    /// rent-exempt reserve cannot be withdrawn (use `close` for that).
    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        ctx.accounts.withdraw(amount)
    }

    /// Drains the vault back to the user and closes `VaultState`, refunding
    /// all rent.
    pub fn close(ctx: Context<Close>) -> Result<()> {
        ctx.accounts.close()
    }
}
