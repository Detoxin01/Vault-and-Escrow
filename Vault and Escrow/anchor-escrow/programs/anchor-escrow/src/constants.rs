use anchor_lang::prelude::*;

/// Seed for the escrow PDA: `[ESCROW_SEED, maker, seed.to_le_bytes()]`.
#[constant]
pub const ESCROW_SEED: &[u8] = b"escrow";

/// Upper bound for how long an offer may stay open (1 year, in seconds).
#[constant]
pub const MAX_DURATION: i64 = 365 * 24 * 60 * 60;

/// `expires_at` value meaning "this offer never expires".
#[constant]
pub const NO_EXPIRY: i64 = 0;
