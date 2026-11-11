# ⚡ Turbine 101 — My Turbin3 Solana Builder Journey

Hey there! 👋 I'm **Rastii**, and this repository is the home for everything I'm building, testing, and shipping during the **Turbin3 Solana Builders Fellowship**.

Instead of spreading my work across dozens of disconnected repos, I organized this space as an open monorepo of independent mini-projects, assignments, and smart contract experiments. Feel free to explore how each project evolves as we progress through the curriculum.

---

## 🗺️ What's Inside

Here is a quick snapshot of what I've tackled so far:

* [**Week 1 — Solana Smoker (SPL & Metaplex Core)**](solana_smoker/)  
  Deep dive into tokenomics on Solana: creating fungible SPL tokens with `@solana/kit`, and exploring the entire lifecycle of Metaplex Core NFTs (uploading to Irys, minting, updating metadata, transferring, and burning to reclaim rent).

* [**Week 2 — Anchor Vault (`q4_26_vault`)**](anchor_vault_starter_q4_26/)  
  On-chain SOL vault program built with **Rust** and **Anchor**, featuring deterministic `VaultState` and `SystemAccount` PDAs across `initialize`, `deposit`, `withdraw`, and `close` instructions, verified end-to-end with **LiteSVM**.

* [**Week 2 — Custom Timed Escrow (`my_custom_escrow_q4`)**](my_custom_escrow_q4/)  
  Timed conditional SPL Token & Token-2022 escrow built with **Rust** and **Anchor**, supporting `make`, `take`, `refund`, and `update` with `Clock`-enforced unlock/expiry windows and optional arbiter co-signatures, verified end-to-end with **LiteSVM**.

---

## 🚀 How to Run Any Project

Each folder in here is completely self-contained with its own dependencies and configuration:

1. Jump into any project folder:
   ```bash
   cd <project-folder>
   ```
2. Follow the quick setup and run instructions in that project's local `README.md`.

*Note: All private wallets, keys, and heavy build artifacts (`target/`, `node_modules/`) are gitignored globally to keep this repository clean and safe.*
