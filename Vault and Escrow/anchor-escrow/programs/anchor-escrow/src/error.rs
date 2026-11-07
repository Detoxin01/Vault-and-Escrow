use anchor_lang::prelude::*;

#[error_code]
pub enum EscrowError {
    #[msg("Amount must be greater than zero")]
    InvalidAmount,
    #[msg("Duration must be between 0 (no expiry) and MAX_DURATION seconds")]
    InvalidDuration,
    #[msg("Invalid time window: unlock delay must be >= 0 and unlock must precede expiry")]
    InvalidTimeWindow,
    #[msg("Attaching a verifier condition requires a non-zero expiry duration")]
    VerifierRequiresExpiry,
    #[msg("The escrow timelock has not unlocked yet")]
    EscrowLocked,
    #[msg("The escrow has expired and can no longer be taken or confirmed")]
    EscrowExpired,
    #[msg("Only the maker can refund before the escrow expires")]
    NotExpired,
    #[msg("Condition not met: delivery confirmation or verifier co-signature is required")]
    ConditionNotMet,
    #[msg("Signer does not match the escrow's designated verifier")]
    UnauthorizedVerifier,
    #[msg("Delivery has already been confirmed")]
    AlreadyConfirmed,
    #[msg("Delivery is confirmed; maker cannot refund until the escrow expires")]
    DeliveryConfirmedActive,
    #[msg("The escrow terms changed; the receive amount does not match")]
    ReceiveMismatch,
    #[msg("Mint A and mint B must be different")]
    SameMint,
    #[msg("Arithmetic overflow")]
    Overflow,
}
