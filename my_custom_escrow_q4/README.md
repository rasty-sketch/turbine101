# ⏳ Custom Timed Escrow (`my_custom_escrow_q4`) — Week 2 Turbin3 Builder Journey

Welcome to my Week 2 Turbin3 Builders escrow project! `my_custom_escrow_q4` is a **timed, conditional token escrow** program written in **Rust** with **Anchor** (`1.1.2`), supporting both **SPL Token** and **Token-2022** cross-standard swaps via `anchor_spl::token_interface`, and tested end-to-end in-memory with **LiteSVM** (`0.10.0`).

Beyond a standard atomic swap, this escrow leverages the on-chain **`Clock`** sysvar to enforce an active settlement window (`[unlock_time, deadline]`) alongside an optional third-party **`arbiter`** co-signature condition on settlement.

---

## 🏗️ Architecture & Account Design

| Account | Type | Seeds / Derivation | Purpose |
| :--- | :--- | :--- | :--- |
| `escrow` | `Account<'info, Escrow>` | `[b"escrow", user.key().as_ref(), &escrow_id.to_le_bytes()]` | Stores maker `authority`, `amount`, `cost`, `escrow_id`, `unlock_time`, `deadline`, `offered_mint`, `wanted_mint`, optional `arbiter`, and canonical PDA `bump`. |
| `escrow_ata` | `InterfaceAccount<'info, TokenAccount>` | ATA of `escrow` PDA for `offered_mint` | Non-custodial vault ATA holding the maker's offered tokens until `take` or `refund`, then closed to return rent lamports to `user`. |

### Security & Edge-Case Hardening
- **Dual `TokenInterface` Support**: Separate `offered_token_program` and `wanted_token_program` interfaces allow seamless swaps between legacy SPL Token and Token-2022 mints.
- **Dust-Griefing Immunity**: Both `take` and `refund` sweep the full `escrow_ata.amount` before calling `close_account`, preventing attackers from bricking vault closure by sending `1` base unit of dust to `escrow_ata`.
- **On-Demand Maker ATA Initialization**: `user_wanted_ata` is initialized `init_if_needed` during `take` (rather than `make`), saving makers upfront rent and preventing makers from bricking `take` by closing an empty ATA.

---

## ⚙️ Program Instructions

1. **`make(amount, cost, unlock_time, deadline, escrow_id, arbiter)`** (`src/lib.rs`)  
   Validates `amount > 0`, `cost > 0`, `offered_mint != wanted_mint`, `deadline > clock.unix_timestamp`, and `0 <= unlock_time < deadline`. Initializes the `escrow` PDA and `escrow_ata`, then transfers `amount` of `offered_mint` from the maker via `transfer_checked`.

2. **`take(escrow_id)`** (`src/lib.rs`)  
   Verifies `unlock_time <= clock.unix_timestamp <= deadline` via the `Clock` sysvar, ensures the taker is not the maker, and enforces co-signing by `arbiter` whenever `escrow.arbiter` is `Some(pubkey)`. Transfers `cost` of `wanted_mint` from buyer to maker, sweeps `escrow_ata` to buyer, and closes both `escrow_ata` and `escrow` back to `user`.

3. **`refund(escrow_id)`** (`src/lib.rs`)  
   Allows the maker (`authority`) to cancel and exit anytime prior to settlement (before `unlock_time`, during the active window, or after `deadline`), returning the full vault token balance and reclaiming all rent lamports from `escrow_ata` and `escrow`.

4. **`update(escrow_id, new_amount, new_cost, new_unlock_time, new_deadline, new_arbiter)`** (`src/lib.rs`)  
   Lets the maker update escrow terms on the fly—adjusting deposited `amount` (depositing the delta or partially withdrawing via PDA signer seeds), changing `cost`, extending/updating `[unlock_time, deadline]`, or adding/removing the `arbiter` condition.

---

## 🧪 LiteSVM Test Suite (`programs/my_custom_escrow_q4/tests/test_escrow.rs`)

- **`test_make_and_take_full_lifecycle`**: Full `make` → `take` swap, on-demand ATA creation, and rent reclamation.
- **`test_timed_escrow_clock_unlock_and_deadline_with_update_extension`**: `Clock` time-travel testing `EscrowLocked` before `unlock_time`, `Expired` past `deadline`, deadline extension via `update`, and valid settlement inside the updated window.
- **`test_arbiter_signature_condition`**: Verifies rejection of missing/unauthorized arbiters (`MissingArbiter`, `InvalidArbiter`) and settlement with valid `arbiter` co-signature.
- **`test_update_amount_cost_schedule_and_arbiter`**: Tests deposit top-ups, partial withdrawals, price/schedule/arbiter updates, unauthorized caller rejection, and empty update rejection.
- **`test_refund_before_or_after_deadline_and_dust_griefing_immunity`**: Verifies unauthorized refund rejection, dust-token griefing immunity on `close_account`, and refunds before unlock and after expiry.
- **`test_cross_standard_swap_spl_token_for_token_2022`**: End-to-end swap between legacy SPL Token and Token-2022.
- **`test_validation_guards`**: Boundary checks for zero amount/cost, `SameMint`, invalid schedules, and self-take prevention.

---

## 🛠️ Prerequisites

- **Rust**: `1.89.0` (pinned via [`rust-toolchain.toml`](rust-toolchain.toml))
- **Solana CLI**: `2.x` / `3.x` (`solana-keygen`, SBF toolchain)
- **Anchor CLI**: `1.1.2` (or via `avm`)

> **Wallet Note:** No private keys or `*-keypair.json` files are committed to this repository. Ensure you have a local Solana keypair at `~/.config/solana/id.json` (`solana-keygen new`) if deploying to localnet or devnet. If you generate a fresh program keypair locally, run `anchor keys sync` before deploying.

---

## 🚀 Build & Test

From inside this directory:

```bash
cd my_custom_escrow_q4

# Compile the SBF program (.so) and generate IDL/types
anchor build

# Run the LiteSVM integration test suite (7 tests)
anchor test
```
