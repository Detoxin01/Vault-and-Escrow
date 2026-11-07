
export const MockMarketErrorCode = {
  InvalidAmount: 6000,
  InsufficientLiquidity: 6001
};

export type MockMarketErrorName = keyof typeof MockMarketErrorCode;
