    import { createUmi } from "@metaplex-foundation/umi-bundle-defaults";
    import wallet from "../../devnet-wallet.json";
    import {
      createSignerFromKeypair,
      publicKey,
      signerIdentity,
    } from "@metaplex-foundation/umi";
    import { fetchAsset, mplCore, update } from "@metaplex-foundation/mpl-core";
    import { base58 } from "@metaplex-foundation/umi/serializers";

    const umi = createUmi(
      process.env.SOLANA_RPC_URL ?? "https://api.devnet.solana.com",
    );
    
    const keypair = umi.eddsa.createKeypairFromSecretKey(new Uint8Array(wallet));
    const signer = createSignerFromKeypair(umi, keypair);

    umi.use(signerIdentity(signer));
    umi.use(mplCore());

    (async () => {
        try{
            const assetAddress = publicKey("8TiyA3E3w2LvEg7omTFshdf2GGn19LnRbSJJZK1TTmov");
            const asset = await fetchAsset(umi,assetAddress);

            const tx = await update(umi, {
                asset,
                name: "Smoking That Solana Pack",
                uri: asset.uri
            }).sendAndConfirm(umi);

            const signature = base58.deserialize(tx.signature)[0];
            console.log(`Updated Signature: ${signature}`);
        } catch (e) {
            console.log(`Error : ${e}`);
        }
    })();