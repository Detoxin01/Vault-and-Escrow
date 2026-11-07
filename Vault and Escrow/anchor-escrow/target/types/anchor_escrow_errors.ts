
export const AnchorEscrowErrorCode = {
  InvalidAmount: 6000,
  InvalidDuration: 6001,
  InvalidTimeWindow: 6002,
  VerifierRequiresExpiry: 6003,
  EscrowLocked: 6004,
  EscrowExpired: 6005,
  NotExpired: 6006,
  ConditionNotMet: 6007,
  UnauthorizedVerifier: 6008,
  AlreadyConfirmed: 6009,
  DeliveryConfirmedActive: 6010,
  ReceiveMismatch: 6011,
  SameMint: 6012,
  Overflow: 6013
};

export type AnchorEscrowErrorName = keyof typeof AnchorEscrowErrorCode;
