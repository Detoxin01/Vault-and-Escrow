use crate::{constants::ESCROW_SEED, error::EscrowError, state::Escrow};
use anchor_lang::prelude::*;

/// Lets the maker revise the requested `receive` amount, time window, and/or
/// verifier condition without closing and re-creating the escrow.
#[derive(Accounts)]
pub struct Update<'info> {
    pub maker: Signer<'info>,

    #[account(
        mut,
        has_one = maker,
        seeds = [ESCROW_SEED, maker.key().as_ref(), escrow.seed.to_le_bytes().as_ref()],
        bump = escrow.bump,
    )]
    pub escrow: Account<'info, Escrow>,
}

impl<'info> Update<'info> {
    pub fn update(
        &mut self,
        new_receive: u64,
        new_unlock_delay: i64,
        new_duration: i64,
        new_verifier: Option<Pubkey>,
    ) -> Result<()> {
        require!(new_receive > 0, EscrowError::InvalidAmount);
        if self.escrow.verifier.is_some() && self.escrow.delivery_confirmed {
            return err!(EscrowError::AlreadyConfirmed);
        }

        let now = Clock::get()?.unix_timestamp;
        let (unlock_at, expires_at) = Escrow::compute_window(
            now,
            new_unlock_delay,
            new_duration,
            new_verifier.is_some(),
        )?;

        self.escrow.receive = new_receive;
        self.escrow.unlock_at = unlock_at;
        self.escrow.expires_at = expires_at;
        self.escrow.delivery_confirmed = new_verifier.is_none();
        self.escrow.confirmed_at = 0;
        self.escrow.verifier = new_verifier;
        Ok(())
    }
}
