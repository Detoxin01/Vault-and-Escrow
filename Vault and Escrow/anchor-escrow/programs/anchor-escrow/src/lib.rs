pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("BS4oJrc7JBdHoijrqa8nsdF8T3x9EsNxzBdhpQCinkpB");

#[program]
pub mod anchor_escrow {
    use super::*;

    /// Maker locks `deposit` of `mint_a` in `vault` and asks for `receive` of
    /// `mint_b`, governed by a `Clock` time window (`unlock_delay`, `duration`)
    /// and an optional `verifier` (delivery confirmation / co-signature).
    pub fn make(
        ctx: Context<Make>,
        seed: u64,
        deposit: u64,
        receive: u64,
        unlock_delay: i64,
        duration: i64,
        verifier: Option<Pubkey>,
    ) -> Result<()> {
        ctx.accounts
            .init_escrow(seed, receive, unlock_delay, duration, verifier, &ctx.bumps)?;
        ctx.accounts.deposit(deposit)
    }

    /// Designated `verifier` confirms delivery on-chain before `expires_at`.
    pub fn confirm_delivery(ctx: Context<ConfirmDelivery>) -> Result<()> {
        ctx.accounts.confirm_delivery()
    }

    /// Taker pays `receive` of `mint_b` to `maker` and receives the full vault
    /// of `mint_a`. Requires unlocked & unexpired `Clock` window, satisfied
    /// verifier condition (prior `confirm_delivery` or inline `verifier`
    /// co-signature), and matching `expected_receive`.
    pub fn take(ctx: Context<Take>, expected_receive: u64) -> Result<()> {
        ctx.accounts.check(expected_receive)?;
        ctx.accounts.pay_maker()?;
        ctx.accounts.withdraw_and_close_vault()
    }

    /// Returns the deposit to `maker` and closes `vault` + `escrow`.
    /// Maker: before delivery confirmation, or after expiry.
    /// Anyone (crank): after expiry.
    pub fn refund(ctx: Context<Refund>) -> Result<()> {
        ctx.accounts.check()?;
        ctx.accounts.refund_and_close_vault()
    }

    /// Maker updates `receive`, `unlock_delay`, `duration`, and `verifier`
    /// (allowed anytime prior to delivery confirmation).
    pub fn update(
        ctx: Context<Update>,
        new_receive: u64,
        new_unlock_delay: i64,
        new_duration: i64,
        new_verifier: Option<Pubkey>,
    ) -> Result<()> {
        ctx.accounts
            .update(new_receive, new_unlock_delay, new_duration, new_verifier)
    }
}
