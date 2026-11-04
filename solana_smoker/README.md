# 🚬 Solana Smoker — Week 1 Turbin3 Builder Journey

Hey! Welcome to my Week 1 Turbin3 assignment. The goal here was to get hands-on with Solana fundamentals: understanding how fungible tokens actually work under the hood, and diving into the new Metaplex Core standard for NFTs.

Everything in here runs against Solana Devnet and is broken down into simple, step-by-step scripts.

---

## 🛠️ Getting Started

Before running anything, you'll need a funded devnet wallet and an image:

1. **Add your wallet**: Drop your devnet keypair into the root of the project as `devnet-wallet.json` (don't worry, it's gitignored!).
2. **Install dependencies**:
   ```bash
   npm install
   ```
3. **Pick an image**: Place your NFT artwork in the root as `smoking__solana_pack.png` (or change the path in `src/nft/nft_image.ts`).

---

## 🪙 Part 1: SPL Token Lifecycle

For this part, I used the modern `@solana/kit` and `@solana-program/token` libraries alongside Metaplex Token Metadata V3 to bring a custom fungible token to life.

Here is the flow:

1. **Initialize the Mint (`npm run spl:init`)**  
   Creates a brand new token mint on-chain, configures rent exemption, sets 6 decimal places, and designates your wallet as the mint authority.

2. **Add Metadata (`npm run spl:metadata`)**  
   Attaches a human-readable name ("Solana Smoker") and symbol ("SS") to the mint using Metaplex Token Metadata V3 via Umi.

3. **Mint Your Tokens (`npm run spl:mint`)**  
   Finds or creates your Associated Token Account (ATA) and mints 100 tokens straight into your balance.

4. **Transfer Tokens (`npm run spl:transfer`)**  
   Derives the recipient's ATA (creating it on the fly if needed) and sends tokens across accounts.

---

## 🎨 Part 2: Metaplex Core NFT Lifecycle

Metaplex Core is Metaplex's latest standard—it drops the old multi-account overhead (master editions, token records, etc.) down to a single clean account model, drastically cutting transaction costs and rent.

Here's how the full lifecycle plays out:

1. **Upload Assets to Irys (`npm run nft:image` & `npm run nft:metadata`)**  
   First uploads the artwork to Irys decentralized storage, then generates and uploads the Metaplex Core-compliant JSON metadata containing the attributes and image URI.

2. **Mint the Core Asset (`npm run nft:mint`)**  
   Mints the NFT on-chain with a single transaction pointing to the metadata URI.

3. **Update Authority in Action (`npm run nft:update`)**  
   Demonstrates dynamic metadata updates by changing the asset's name to *"Smoking That Solana Pack"* using your update authority rights.

4. **Transfer Ownership (`npm run nft:transfer`)**  
   Safely transfers asset ownership to a new wallet address.

5. **Burn & Reclaim Rent (`npm run nft:burn`)**  
   The best part about Solana accounts: when you're done with an asset, you can permanently burn it and reclaim the locked rent lamports straight back into your wallet!
