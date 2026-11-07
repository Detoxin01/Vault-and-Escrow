# Anchor SOL Vault (`anchor-vault`)

A native SOL vault program built with **Anchor 1.2.2** and tested end-to-end in **TypeScript** using **LiteSVM**.

Each user gets two deterministic Program Derived Addresses (PDAs):
- **`vault_state`** (`seeds = [b"state", user.key().as_ref()]`) — an Anchor account owned by the program that stores canonical PDA bumps (`state_bump`, `vault_bump`).
- **`vault`** (`seeds = [b"vault", user.key().as_ref()]`) — a zero-data `SystemAccount` PDA that holds the user's lamports and signs outbound transfers via `CpiContext::new_with_signer`.

---

## Architecture

```
User Wallet (Signer)
   │
   ├── initialize ──► Creates VaultState PDA + funds Vault PDA with rent-exempt minimum
   ├── deposit    ──► SystemProgram::transfer (User ──► Vault PDA)
   ├── withdraw   ──► SystemProgram::transfer signed by Vault PDA (keeps rent-exempt reserve)
   └── close      ──► Drains 100% of Vault PDA lamports + closes VaultState PDA (refunds all rent)
```

### Why keep a separate `SystemAccount` PDA for lamports?
In Solana, only the owner program of an account with data may debit its lamports directly, whereas a `SystemAccount` PDA (`0` bytes of data, owned by the System Program) can sign native `SystemProgram::transfer` CPIs cleanly using `CpiContext::new_with_signer`. Separating `VaultState` (program-owned metadata) from `Vault` (system-owned lamports) keeps accounting explicit and idiomatic.

---

## Instructions

| Instruction | Parameters | Description |
| :--- | :--- | :--- |
| `initialize` | — | Initializes `vault_state` and transfers the exact `Rent::get()?.minimum_balance(0)` lamports from `user` to `vault` so the account is rent-exempt on-chain. |
| `deposit` | `amount: u64` | Requires `amount > 0` and transfers `amount` lamports from `user` into `vault`. |
| `withdraw` | `amount: u64` | Requires `amount > 0` and `amount <= vault.lamports() - rent_exempt`. Transfers `amount` lamports from `vault` back to `user` using PDA signer seeds while preserving rent-exemption. |
| `close` | — | Transfers **all** lamports in `vault` (deposits + rent reserve) to `user` and closes `vault_state` (`close = user`), returning 100% of rent so zero lamports are stranded. |

---

## Custom Errors

| Code | Name | Condition |
| :--- | :--- | :--- |
| `6000` | `InvalidAmount` | Deposit or withdrawal `amount == 0`. |
| `6001` | `InsufficientFunds` | Withdrawal `amount` exceeds `vault.lamports() - rent_exempt`. |

---

## Project Structure

```text
anchor-vault/
├── Anchor.toml
├── package.json
├── tsconfig.json
├── programs/anchor-vault/src/
│   ├── lib.rs
│   ├── constants.rs
│   ├── error.rs
│   ├── state.rs
│   └── instructions/
│       ├── initialize.rs
│       ├── deposit.rs
│       ├── withdraw.rs
│       └── close.rs
└── tests/
    ├── utils.ts          # LiteSVM + Anchor TS test harness
    └── anchor-vault.ts   # Full TypeScript + LiteSVM test suite
```

---

## Build & Run Tests (WSL2 / Linux / macOS)

```bash
# 1. Build SBF binary + IDL + TypeScript types
anchor build --arch v0

# 2. Install TypeScript dependencies
yarn install

# 3. Run TypeScript + LiteSVM test suite
yarn test
# or via Anchor CLI (skip_local_validator = true):
ANCHOR_BUILD_SBF_ARCH=v0 anchor test --skip-build
```
