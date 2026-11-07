use anchor_lang::prelude::*;

/// Vault PDA: `[VAULT_SEED, manager, market]`.
#[constant]
pub const VAULT_SEED: &[u8] = b"vault";

/// Share mint PDA: `[SHARES_SEED, vault]`.
#[constant]
pub const SHARES_SEED: &[u8] = b"shares";
