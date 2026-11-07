use crate::{
    constants::{MAX_DURATION, NO_EXPIRY},
    error::EscrowError,
};
use anchor_lang::prelude::*;

/// Timed & conditional SPL token escrow.
///
/// The maker locks `mint_a` tokens inside `vault` and specifies:
/// 1. `receive`: how much `mint_b` the taker must pay.
/// 2. `unlock_at` & `expires_at`: a `Clock`-enforced settlement time window.
/// 3. `verifier`: an optional third-party authority (e.g., courier, oracle, or
///    arbiter) whose condition must be satisfied either via a prior
///    `confirm_delivery` transaction or by co-signing `take` directly.
#[account]
#[derive(InitSpace)]
pub struct Escrow {
    /// User-chosen seed so one maker can run many escrows concurrently.
    pub seed: u64,
    pub maker: Pubkey,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    /// Amount of `mint_b` the maker wants in exchange.
    pub receive: u64,
    /// Unix timestamp (from the `Clock` sysvar) when the escrow was created.
    pub created_at: i64,
    /// Earliest Unix timestamp when `take` is allowed (`0` = unlocked immediately).
    pub unlock_at: i64,
    /// Deadline Unix timestamp after which `take` is rejected and anyone may
    /// permissionlessly `refund` the deposit back to the maker (`0` = no expiry).
    pub expires_at: i64,
    /// Optional verifier / delivery-confirmation authority. `None` = unconditional.
    pub verifier: Option<Pubkey>,
    /// True once `confirm_delivery` is executed (or immediately if `verifier` is `None`).
    pub delivery_confirmed: bool,
    /// Unix timestamp when delivery was confirmed (`0` if not yet confirmed).
    pub confirmed_at: i64,
    pub bump: u8,
}

impl Escrow {
    pub fn is_locked(&self, now: i64) -> bool {
        self.unlock_at > 0 && now < self.unlock_at
    }

    pub fn is_expired(&self, now: i64) -> bool {
        self.expires_at != NO_EXPIRY && now >= self.expires_at
    }

    /// Computes `(unlock_at, expires_at)` from relative delays and validates invariants.
    pub fn compute_window(
        now: i64,
        unlock_delay: i64,
        duration: i64,
        has_verifier: bool,
    ) -> Result<(i64, i64)> {
        require!(unlock_delay >= 0, EscrowError::InvalidTimeWindow);
        require!(
            (0..=MAX_DURATION).contains(&duration),
            EscrowError::InvalidDuration
        );
        if has_verifier {
            require!(duration > 0, EscrowError::VerifierRequiresExpiry);
        }

        let unlock_at = if unlock_delay == 0 {
            0
        } else {
            now.checked_add(unlock_delay)
                .ok_or(error!(EscrowError::Overflow))?
        };

        let expires_at = if duration == 0 {
            NO_EXPIRY
        } else {
            now.checked_add(duration)
                .ok_or(error!(EscrowError::Overflow))?
        };

        if unlock_at > 0 && expires_at != NO_EXPIRY {
            require!(expires_at > unlock_at, EscrowError::InvalidTimeWindow);
        }

        Ok((unlock_at, expires_at))
    }
}
