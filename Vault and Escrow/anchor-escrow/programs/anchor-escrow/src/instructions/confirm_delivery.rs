use crate::{constants::ESCROW_SEED, error::EscrowError, state::Escrow};
use anchor_lang::prelude::*;

/// Designated `verifier` records on-chain delivery / milestone confirmation
/// before the escrow's `Clock` expiry, unlocking settlement for the taker.
#[derive(Accounts)]
pub struct ConfirmDelivery<'info> {
    pub verifier: Signer<'info>,

    #[account(
        mut,
        seeds = [ESCROW_SEED, escrow.maker.as_ref(), escrow.seed.to_le_bytes().as_ref()],
        bump = escrow.bump,
    )]
    pub escrow: Account<'info, Escrow>,
}

impl<'info> ConfirmDelivery<'info> {
    pub fn confirm_delivery(&mut self) -> Result<()> {
        let expected = self
            .escrow
            .verifier
            .ok_or(error!(EscrowError::AlreadyConfirmed))?;
        require_keys_eq!(
            self.verifier.key(),
            expected,
            EscrowError::UnauthorizedVerifier
        );
        require!(
            !self.escrow.delivery_confirmed,
            EscrowError::AlreadyConfirmed
        );

        let now = Clock::get()?.unix_timestamp;
        require!(!self.escrow.is_expired(now), EscrowError::EscrowExpired);

        self.escrow.delivery_confirmed = true;
        self.escrow.confirmed_at = now;
        Ok(())
    }
}
