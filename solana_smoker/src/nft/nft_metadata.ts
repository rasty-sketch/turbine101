import {
  createSignerFromKeypair,
  signerIdentity,
} from "@metaplex-foundation/umi";
import wallet from "../../devnet-wallet.json";
import { createUmi } from "@metaplex-foundation/umi-bundle-defaults";
import { irysUploader } from "@metaplex-foundation/umi-uploader-irys";

const umi = createUmi(
  process.env.SOLANA_RPC_URL ?? "https://api.devnet.solana.com",
);

const keypair = umi.eddsa.createKeypairFromSecretKey(new Uint8Array(wallet));
const signer = createSignerFromKeypair(umi, keypair);

umi.use(
  irysUploader({
    address: "https://devnet.irys.xyz/",
  }),
);

umi.use(signerIdentity(signer));

(async () => {
  try {
    //change the image uri to your image uri obtained from nft_image.ts
    const image =
      "https://gateway.irys.xyz/DvGzY79HgwPyYBc6jr5sRtTCNTXGo8FpQ4GSV9JWvYs7";

    //json scheme : https://www.metaplex.com/docs/smart-contracts/core/json-schema
    //change the metadata
    const metadata = {
      name: "Solana Smoker",
      description: "Some has really cooked and mastered NFTs, therefore smoking that solana pack",
      image,
            external_url: "https://turbin3.com",
      attributes: [
        {
          trait_type: "Flavor",
          value: "Solana",
        },
      ],
      properties: {
        files: [
          {
            uri: image,
            type: "image/png",
          },
        ],
        category: "image",
      },
    };

    const myUri = await umi.uploader.uploadJson(metadata);
    console.log(`metadata uri: ${myUri}`);
  } catch (error) {
    console.log("error", error);
  }
})();
