# Timed & Conditional SPL Token Escrow (`anchor-escrow`)

An SPL Token / Token-2022 compatible escrow program built with **Anchor 1.2.2** (`InterfaceAccount` / `transfer_checked`) and tested in **TypeScript** with **LiteSVM**, covering:
1. Full **`make` / `take` / `refund` / `update`** instruction lifecycle.
2. **Timed Escrow (`Clock` sysvar)** enforcing both an optional **unlock cliff (`unlock_at`)** and an **expiration deadline (`expires_at`)** with permissionless crank refunds after expiry.
3. **Conditional Release (`verifier` signature / delivery confirmation)** supporting **both** two-step on-chain attestation (`confirm_delivery`) and single-transaction atomic co-signing on `take`.

---

## Architecture & Settlement Flows

```
Maker ──► make(seed, deposit, receive, unlock_delay, duration, verifier)
            │
            ├── Initializes Escrow PDA: seeds = ["escrow", maker, seed.to_le_bytes()]
            └── Transfers `deposit` of Mint A ──► Vault ATA (authority = Escrow PDA)

                     ┌──────────────────────────────────────────────────────────┐
                     │        Clock Sysvar Window: [unlock_at, expires_at)      │
                     └──────────────────────────────────────────────────────────┘
                                   │                              │
            Condition Mode A:      │                              │  Condition Mode B:
   Verifier calls confirm_delivery │                              │  Verifier co-signs `take`
        (sets delivery_confirmed)  │                              │  inline in a single tx
                                   ▼                              ▼
                            Taker calls take(expected_receive)
                                   │
                                   ├── Taker pays `receive` of Mint B ──► Maker ATA B
                                   ├── Vault sends all Mint A ──────────► Taker ATA A
                                   └── Closes Vault + Escrow (rent refunded to Maker)

   If `now >= expires_at` (or Maker cancels before confirmation):
   Caller / Maker calls refund() ──► Vault returns all Mint A + rent strictly to Maker
```

---

## Key Design Highlights

- **Dual-Mode Verifier Condition (`confirm_delivery` + Inline Co-Signature)**:
  - Attach `verifier: Some(pubkey)` (e.g., a delivery courier, escrow arbiter, or oracle) to require third-party authorization before settlement.
  - **Mode A (Asynchronous Delivery Confirmation)**: `verifier` submits `confirm_delivery` before `expires_at`. Once confirmed, `maker` is blocked from hiking the price (`update`) or pulling the rug (`refund`) while the settlement window is active.
  - **Mode B (Atomic Co-Signature)**: `verifier` co-signs `take` directly alongside `taker`, settling in one transaction without an extra state write.
- **Guaranteed Liveness (Zero Frozen Funds)**:
  - Attaching a `verifier` requires `duration > 0` (`VerifierRequiresExpiry`). Even if delivery is confirmed and the taker disappears, once `Clock::get()?.unix_timestamp >= expires_at`, the maker (or any permissionless crank) can call `refund` to recover the tokens and account rent.
- **Front-Running Protection on `update`**:
  - Because `update` lets the maker revise `receive`, `take` takes an `expected_receive: u64` argument and aborts with `ReceiveMismatch` if the maker front-ran the taker with a worse price.

---

## Instructions

| Instruction | Signer(s) | Description |
| :--- | :--- | :--- |
| `make` | `maker` | Creates `escrow` PDA + `vault` ATA, validates `Clock` window and optional `verifier`, and locks `deposit` of `mint_a`. |
| `confirm_delivery` | `verifier` | Designated verifier records `delivery_confirmed = true` and `confirmed_at = now` prior to `expires_at`. |
| `take` | `taker` (+ optional `verifier`) | Verifies `now >= unlock_at`, `now < expires_at`, condition met, and `receive == expected_receive`; swaps `mint_b` for `mint_a` and closes `vault` + `escrow`. |
| `update` | `maker` | Updates `receive`, `unlock_at`, `expires_at`, and `verifier` (disallowed once delivery has been confirmed). |
| `refund` | `caller` (`maker` or crank) | Returns `mint_a` and rent to `maker`. Maker: anytime before confirmation, or after expiry. Anyone: permissionlessly once `now >= expires_at`. |

---

## Build & Run Tests (WSL2 / Linux / macOS)

```bash
# 1. Build SBF binary + IDL + TypeScript types
anchor build --arch v0

# 2. Install TypeScript dependencies
yarn install

# 3. Run TypeScript + LiteSVM test suite
yarn test
```
