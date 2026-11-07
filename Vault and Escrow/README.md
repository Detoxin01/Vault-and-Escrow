# Week 2 Assignment — Vault & Escrow (Q4 2026 Builders Cohort)

Complete implementation of all **Core Tasks (1–3)**, the **Timed & Conditional Escrow Extension (Task 4)**, and the **Non-Custodial In-Kind Vault Advanced Extension (Task 5)** built with **Anchor 1.2.2** and tested in **TypeScript** using **LiteSVM**.

---

## Repository Map

| Directory | Assignment Task(s) | Highlights |
| :--- | :--- | :--- |
| [`./anchor-vault`](./anchor-vault/README.md) | **Task 1 & Task 3** | Native SOL Vault with `initialize`, `deposit`, `withdraw` (preserving rent-exemption), and `close` (draining vault + refunding state rent). Full TypeScript + LiteSVM test suite. |
| [`./anchor-escrow`](./anchor-escrow/README.md) | **Task 2, Task 3 & Task 4 (Extension)** | SPL Token Escrow with `make`, `take`, `refund`, `update` **plus** `Clock`-enforced `[unlock_at, expires_at)` time windows, permissionless post-expiry crank refunds, and dual-mode **Verifier Delivery Confirmation (`confirm_delivery`) / Inline Co-Signature**. |
| [`./anchor-nc-vault`](./anchor-nc-vault/README.md) | **Task 5 (Advanced Extension) & Task 3** | **Non-Custodial Share Vault** + **Underlying Lending Market (`mock-market`)**. Manager can only rebalance between vault-owned accounts; users can **exit anytime** via liquid `withdraw` or `redeem_in_kind` (receiving underlying market position tokens even when both the vault and market have **zero liquidity**). |

---

## Running All Tests (Quick Start)

Inside **WSL2 (Ubuntu)** / Linux / macOS:

```bash
# Run individual project tests:
cd anchor-vault    && yarn test
cd ../anchor-escrow   && yarn test
cd ../anchor-nc-vault && yarn test
```

Or run every suite back-to-back with `./run-all-tests.sh` (perfect for capturing a single terminal screenshot of all tests passing):

```bash
bash ./run-all-tests.sh
```

---

## Social Thread / Article Draft (For Submission)

Below is a ready-to-post X thread / LinkedIn post explaining what was built, key Solana design lessons, and real-world DeFi applications:

> **1/6** Just shipped Week 2 of the @Turbin3 Q4 Builders Cohort: **Vaults, Timed Conditional Escrows, and Non-Custodial In-Kind Vaults** on @solana using Anchor + TypeScript LiteSVM!
>
> Here is what I built, what broke my mental model, and why non-custodial exit paths matter in DeFi:
>
> **2/6** **1. Native SOL Vault (`initialize`, `deposit`, `withdraw`, `close`)**
> Separating `VaultState` (program-owned PDA storing canonical bumps) from `Vault` (0-byte SystemAccount PDA holding lamports) lets the program sign native System Program transfers via `CpiContext::new_with_signer` while guarding the rent-exempt minimum on `withdraw` and reclaiming 100% of rent on `close`.
>
> **3/6** **2. Timed & Conditional Escrow (`Clock` Sysvar + Delivery Confirmation)**
> Classic `make / take / refund / update` swaps are great for instant liquidity, but commerce & OTC deals need time bounds and external conditions.
> Using `Clock::get()?.unix_timestamp`, every escrow enforces `[unlock_at, expires_at)` plus an optional `verifier` condition (`confirm_delivery` on-chain OR atomic co-signature on `take`).
>
> **4/6** **Subtle Security Invariant Discovered:**
> Once a `verifier` confirms delivery, the maker shouldn't be able to front-run `update` or rug via `refund` mid-settlement.
> BUT if the taker disappears forever, funds would freeze! Requiring `duration > 0` whenever a verifier is set gives both **fairness during the window** and **guaranteed liveness after `expires_at`** (even via permissionless crank refunds).
>
> **5/6** **3. Non-Custodial Share Vault with In-Kind Redemption**
> What happens when a yield vault deploys 100% of its USDC into a lending pool and borrowers push pool utilization to 100%? Normal `withdraw` reverts — trapping users!
> By tokenizing vault equity as shares and adding `redeem_in_kind`, any user can burn shares at ANY time to claim their exact pro-rata slice of idle tokens + underlying market position tokens (`mUSDC`), with zero liquidity and zero manager permission.
>
> **6/6** Tested all 3 workspaces end-to-end in **TypeScript + LiteSVM** — deterministic `Clock` time-travel (`setClock`), zero validator spin-up overhead, sub-second test runs.
>
> Where this applies in production:
> - Milestone-gated freelance/commerce escrows & OTC RFQ settlement
> - Institutional DeFi vaults (Morpho / Kamino style) with unstoppable user exits during liquidity crunches.
