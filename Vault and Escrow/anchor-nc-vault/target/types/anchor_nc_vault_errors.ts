
export const AnchorNcVaultErrorCode = {
  InvalidAmount: 6000,
  ZeroShares: 6001,
  SlippageExceeded: 6002,
  InsufficientLiquidity: 6003,
  NothingToRedeem: 6004,
  MathOverflow: 6005
};

export type AnchorNcVaultErrorName = keyof typeof AnchorNcVaultErrorCode;
