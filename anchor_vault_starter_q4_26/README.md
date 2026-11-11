# 🔐 Anchor Vault (`q4_26_vault`) — Week 2 Turbin3 Builder Journey

Welcome to my Week 2 Turbin3 Builders assignment! This project is an on-chain **SOL Vault** program written in **Rust** with **Anchor** (`1.1.2`) and tested end-to-end in-memory using **LiteSVM** (`0.10.0`).

Each user gets a deterministic pair of Program Derived Addresses (PDAs) — a `VaultState` account to store canonical PDA bumps and a system-owned `vault` account to custody native SOL safely.

---

## 🏗️ Architecture & Account Design

| Account | Type | Seeds | Purpose |
| :--- | :--- | :--- | :--- |
| `vault_state` | `Account<'info, VaultState>` | `[b"state", user.key().as_ref()]` | Stores `vault_bump` and `state_bump` so subsequent instructions validate PDAs deterministically without recomputing bumps on-chain. |
| `vault` | `SystemAccount<'info>` | `[b"vault", user.key().as_ref()]` | System-owned PDA holding the rent-exempt minimum balance plus any SOL deposited by `user`. |

---

## ⚙️ Program Instructions

1. **`initialize`** (`src/instructions/initialize.rs`)  
   Initializes the `vault_state` PDA, funds the `vault` SystemAccount PDA with the exact rent-exempt minimum lamports via System Program CPI, and records `vault_bump` and `state_bump`.

2. **`deposit(amount: u64)`** (`src/instructions/deposit.rs`)  
   Verifies `amount > 0` (`ErrorCode::InvalidAmount`) and transfers `amount` lamports from the signing `user` into their `vault` PDA.

3. **`withdraw(amount: u64)`** (`src/instructions/withdraw.rs`)  
   Verifies `amount > 0`, ensures `vault.lamports().checked_sub(amount)` remains at or above the rent-exempt minimum balance, and transfers `amount` lamports from the `vault` PDA back to `user` using PDA signer seeds (`[b"vault", user.key().as_ref(), &[vault_state.vault_bump]]`).

4. **`close`** (`src/instructions/close.rs`)  
   Sweeps the entire remaining SOL balance from the `vault` PDA back to `user` via signed CPI and closes `vault_state` (`close = user`), refunding all state rent lamports back to `user`.

---

## 🛠️ Prerequisites

- **Rust**: `1.89.0` (automatically pinned via [`rust-toolchain.toml`](rust-toolchain.toml))
- **Solana CLI**: `2.x` / `3.x` (`solana-keygen`, SBF toolchain)
- **Anchor CLI**: `1.1.2` (or via `avm`)

> **Wallet Note:** No private keys or `*-keypair.json` files are committed to this repository. Ensure you have a local Solana keypair at `~/.config/solana/id.json` (`solana-keygen new`) if deploying to localnet or devnet. If you generate a fresh program keypair locally, run `anchor keys sync` before deploying.

---

## 🚀 Build & Test

From inside this directory:

```bash
cd anchor_vault_starter_q4_26

# Compile the SBF program (.so) and generate IDL/types
anchor build

# Run the LiteSVM integration test suite (Initialize -> Deposit -> Withdraw -> Close)
anchor test
```
