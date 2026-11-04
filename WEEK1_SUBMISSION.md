# Turbin3 Solana Builders Fellowship — Week 1 Submission

**Student:** Rastii (Rasty Amjad)  
**Assignment:** Week 1 — SPL Tokens & Metaplex Core NFT Lifecycle  
**Cohort:** Q4 / 2026 Turbin3 Builders  

---

## 🔗 Submission Links

* **Social Post (X / Twitter Thread):**  
  [https://x.com/Rastyamjad1/status/2118037164168540190](https://x.com/Rastyamjad1/status/2118037164168540190)

* **GitHub Repository (Master Hub):**  
  [https://github.com/rasty-sketch/turbine101](https://github.com/rasty-sketch/turbine101)

* **Week 1 Project Direct Link:**  
  [https://github.com/rasty-sketch/turbine101/tree/main/solana_smoker](https://github.com/rasty-sketch/turbine101/tree/main/solana_smoker)

* **On-Chain Metaplex Core Explorer (Devnet):**  
  [https://core.metaplex.com/explorer/8TiyA3E3w2LvEg7omTFshdf2GGn19LnRbSJJZK1TTmov?env=devnet](https://core.metaplex.com/explorer/8TiyA3E3w2LvEg7omTFshdf2GGn19LnRbSJJZK1TTmov?env=devnet)

---

## 📋 Completed Tasks Checklist

### Core Tasks
- [x] **Task 1: Mint and transfer your own SPL token**
  - Created mint account with 6 decimal places (`src/spl/spl_init.ts`)
  - Initialized on-chain metadata via Metaplex Token Metadata V3 (`src/spl/spl_metadata.ts`)
  - Derived Associated Token Account (ATA) and minted 100 tokens (`src/spl/spl_mint.ts`)
  - Transferred tokens to recipient ATA (`src/spl/spl_transfer.ts`)
- [x] **Task 2: Mint an NFT using MPL Core**
  - Uploaded high-resolution artwork and Core JSON schema to Irys storage (`src/nft/nft_image.ts`, `src/nft/nft_metadata.ts`)
  - Minted single-account Core NFT asset on Solana Devnet (`src/nft/nft_mint.ts`)
- [x] **Task 3: Update NFT name and metadata as update authority**
  - Updated asset name to *"Smoking That Solana Pack"* using update authority permissions (`src/nft/nft_update.ts`)

### Extension Challenges (Optional)
- [x] **Task 4: Transfer NFT ownership between wallets**
  - Transferred asset ownership to recipient wallet via MPL Core `transfer()` instruction (`src/nft/nft_transfer.ts`)
- [x] **Task 5: Permanently destroy the NFT and reclaim rent fees**
  - Burned asset on-chain and reclaimed the locked rent lamports back to the payer (`src/nft/nft_burn.ts`)

---

## 💡 Key Learnings & Takeaways

1. **Solana Account Model vs. EVM:** Unlike EVM where balances exist in a single contract's storage mapping, Solana decouples state into distinct data accounts. Working with Associated Token Accounts (ATAs) and rent exemption makes the mechanics of Solana tokenomics crystal clear.
2. **Metaplex Core Architecture:** Legacy Solana NFTs required multiple auxiliary accounts (Metadata PDA, Master Edition, Token Records), causing higher rent and transaction fees. Metaplex Core condenses the entire asset into a single account, reducing minting and storage costs by ~85%.
3. **Rent Reclaim Mechanism:** Closing/burning unused or expired accounts refunds the locked SOL back to the owner—enabling cost-efficient temporary event passes, consumable in-game items, and low-waste digital assets.
