use anchor_lang::prelude::*;

/// Per-user bookkeeping account. The lamports themselves live in the separate
/// `vault` system-account PDA so that it can sign transfers via the
/// System Program.
#[account]
#[derive(InitSpace)]
pub struct VaultState {
    pub vault_bump: u8,
    pub state_bump: u8,
}
