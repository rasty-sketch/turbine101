    import { createUmi } from "@metaplex-foundation/umi-bundle-defaults";
    import wallet from "../../devnet-wallet.json";
    import {
      createSignerFromKeypair,
      publicKey,
      signerIdentity,
    } from "@metaplex-foundation/umi";
    import { burn, fetchAsset, mplCore } from "@metaplex-foundation/mpl-core";
    import { base58 } from "@metaplex-foundation/umi/serializers";
    
    const umi = createUmi(
      process.env.SOLANA_RPC_URL ?? "https://api.devnet.solana.com",
    );
    
    const keypair = umi.eddsa.createKeypairFromSecretKey(new Uint8Array(wallet));
    const signer = createSignerFromKeypair(umi, keypair);
    
    umi.use(signerIdentity(signer));
    umi.use(mplCore());
    
    const assetAddress = publicKey("AvtDmLJMJz12P3wu8JA4wxZEHM6WByMRtc3ezbz7EdMA");

    (async () => {
      try {
        const asset = await fetchAsset(umi, assetAddress);
        console.log(`Burning asset: ${asset.publicKey} owned by
        ${asset.owner}...`);
    

        const tx = await burn(umi, {
          asset,
        }).sendAndConfirm(umi);

        const signature = base58.deserialize(tx.signature)[0];
        console.log(`NFT permanently burned & rent reclaimed!`);
        console.log(`Burn signature: ${signature}`);
      } catch (error) {
        console.log("Error burning NFT:", error);
      }
    })();