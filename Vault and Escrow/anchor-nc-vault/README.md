# Non-Custodial Share Vault with In-Kind Redemption (`anchor-nc-vault`)

An advanced **non-custodial DeFi vault** + **underlying lending market (`mock-market`)** built with **Anchor 1.2.2** and tested in **TypeScript** with **LiteSVM**.

Designed so that **users retain full custody via vault share tokens and can exit at any time** — even when **100% of vault capital is deployed** AND **the underlying lending market itself has 0% available liquidity** — by redeeming shares **in kind** for their exact pro-rata slice of underlying market position tokens.

---

## The Problem with Traditional Strategy Vaults

In conventional yield vaults:
1. Users deposit `USDC` and the vault manager allocates 100% of `USDC` into an external lending market.
2. Borrowers borrow 100% of the lending market's `USDC` (100% utilization).
3. Neither the vault nor the lending market has liquid `USDC` — **withdrawals revert or enter multi-day queues**, trapping depositors.

## Our Non-Custodial Solution: Dual Exit (`withdraw` + `redeem_in_kind`)

```
                         ┌─────────────────────────────────────┐
   deposit(USDC) ──────► │        Non-Custodial Vault          │ ◄── Manager: allocate / deallocate ONLY
   ◄── mints Share Token │                                     │     (strictly between Vault ATAs &
                         │  • idle ATA          (USDC)         │      the single whitelisted Market)
                         │  • vault_position ATA (mUSDC)       │
                         └──────────┬─────────────────┬────────┘
                                    │                 │
               Liquid Exit          │                 │  Always-Available In-Kind Exit
            withdraw(shares)        │                 │     redeem_in_kind(shares)
        (when idle USDC >= NAV)     ▼                 ▼   (works at ZERO liquidity!)
                              Receives USDC     Receives pro-rata
                                  at NAV        idle USDC + mUSDC
                                                market position tokens
                                                       │
                                                       ▼
                                          User now owns the underlying
                                          `mock_market` position directly
                                          (transfer, LP, or withdraw later)
```

### Why This Design Is Strictly Non-Custodial
1. **No Admin Drain / No Pause / No Lockups**: There is no instruction allowing the `manager` to withdraw tokens to an external wallet, pause redemptions, or freeze user shares.
2. **Immutable Market Binding**: `Vault` commits to a single `market` at initialization (`has_one = market`). `allocate` and `deallocate` only move tokens between `idle` (vault-owned `base_mint` ATA) and `vault_position` (vault-owned `position_mint` ATA) via CPI to `mock_market`.
3. **Unconditional Anytime Exit (`redeem_in_kind`)**: Burning `s` shares out of total supply `S` transfers:
   - $\lfloor \text{idle} \times s / S \rfloor$ of `base_mint` (`USDC`), **and**
   - $\lfloor \text{vault\_position} \times s / S \rfloor$ of `position_mint` (`mUSDC` underlying market receipt tokens)
   directly to the user — requiring **zero vault liquidity, zero market liquidity, zero oracle calls, and zero manager cooperation**.

---

## Workspace Programs

### 1. `anchor-nc-vault` (`programs/anchor-nc-vault`)
| Instruction | Signer | Description |
| :--- | :--- | :--- |
| `initialize` | `manager` | Creates `vault` PDA, `share_mint` PDA, `idle` ATA, and `vault_position` ATA tied to `market`. |
| `deposit` | `user` | Transfers `amount` of `base_mint` into `idle` and mints shares at pre-deposit NAV (`min_shares` slippage guard). |
| `withdraw` | `user` | Burns `shares` and pays full NAV in liquid `base_mint` from `idle` (`min_out` guard; reverts with `InsufficientLiquidity` if `idle < out`). |
| `redeem_in_kind` | `user` | **Always-available exit**: burns `shares` and transfers the user's exact pro-rata share of both `idle` (`base_mint`) and `vault_position` (`position_mint`). |
| `allocate` | `manager` | CPIs into `mock_market::supply` to deploy `idle` base tokens into `vault_position` market tokens. |
| `deallocate` | `manager` | CPIs into `mock_market::withdraw` to unwind `vault_position` back into `idle` base tokens. |

### 2. `mock-market` (`programs/mock-market`)
A minimal lending-pool program representing an external money market (like Kamino / Marginfi / Solend):
- `initialize`: Creates `market` PDA, `position_mint` PDA (`mUSDC` 1:1 receipt token), and `liquidity` ATA.
- `supply` / `withdraw`: Supplies `base_mint` for `position_mint` or burns `position_mint` for `base_mint` (when `liquidity` is available).
- `borrow` / `repay`: Simulates borrower utilization (including 100% utilization where `liquidity.amount == 0`) and subsequent repayment.

---

## Build & Run Tests (WSL2 / Linux / macOS)

```bash
# 1. Build both SBF programs (mock_market + anchor_nc_vault) + IDLs + TS types
anchor build --arch v0

# 2. Install TypeScript dependencies
yarn install

# 3. Run TypeScript + LiteSVM test suite
yarn test
```
