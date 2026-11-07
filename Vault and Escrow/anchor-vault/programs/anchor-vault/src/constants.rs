use anchor_lang::prelude::*;

/// Seed for the per-user `VaultState` PDA (stores the bumps).
#[constant]
pub const STATE_SEED: &[u8] = b"state";

/// Seed for the per-user lamport vault PDA (a plain system account).
#[constant]
pub const VAULT_SEED: &[u8] = b"vault";
