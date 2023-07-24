import * as anchor from "@coral-xyz/anchor";
import { decode, getAccounts, mapProof } from "./utils/utils";
import {
  SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
  SPL_NOOP_PROGRAM_ID,
} from "@solana/spl-account-compression";
import { getAsset, getAssetProof } from "./utils/readAPI";
import { Cutils } from "../target/types/cutils";
import { loadOrGenerateKeypair, loadPublicKeysFromFile } from "./utils/helpers";
import {
  PROGRAM_ID as BUBBLEGUM_PROGRAM_ID,
  Creator,
  Collection,
} from "@metaplex-foundation/mpl-bubblegum/dist/src/generated";
import { PublicKey } from "@metaplex-foundation/js";
import { computeDataHash } from "@metaplex-foundation/mpl-bubblegum";

describe("cutils", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Cutils as anchor.Program<Cutils>;

  const walletSk = [
    218, 120, 169, 174, 4, 65, 67, 185, 168, 63, 46, 229, 131, 19, 104, 111,
    231, 215, 198, 91, 40, 12, 246, 225, 200, 63, 57, 53, 177, 103, 155, 167,
    65, 123, 223, 1, 227, 10, 165, 181, 17, 133, 20, 103, 246, 251, 13, 101,
    195, 4, 164, 241, 129, 206, 150, 40, 134, 188, 178, 42, 186, 111, 111, 122,
  ];
  const payer = anchor.web3.Keypair.fromSecretKey(new Uint8Array(walletSk));

  const uri = "https://arweave.net/nVRvZDaOk5YAdr4ZBEeMjOVhynuv8P3vywvuN5sYSPo";
  const { collectionMint, treeAddress } = loadPublicKeysFromFile();
  console.log("collectionMint", collectionMint);
  console.log("treeAddress", treeAddress);

  it("Verify", async () => {
    // TODO: replace assetId
    const assetId = "4dBQCi6275EmHg4GcutN4XJDuXmhQf7BWN273RgNZLNN";

    const asset = await getAsset(assetId);
    const proof = await getAssetProof(assetId);
    const proofPathAsAccounts = mapProof(proof);
    const root = decode(proof.root);
    const dataHash = decode(asset.compression.data_hash);
    const creatorHash = decode(asset.compression.creator_hash);
    const nonce = new anchor.BN(asset.compression.leaf_id);
    const index = asset.compression.leaf_id;

    const collection: Collection = {
      key: new PublicKey(asset.grouping[0].group_value),
      verified: true,
    };

    const creators: Creator[] = asset.creators.map((creator) => {
      return {
        address: new PublicKey(creator.address),
        verified: creator.verified,
        share: creator.share,
      };
    });
    const message = {
      name: asset.content.metadata?.name || "",
      symbol: asset.content.metadata?.symbol || "",
      uri: asset.content.json_uri,
      sellerFeeBasisPoints: asset.royalty.basis_points,
      creators: creators,
      collection: new PublicKey(asset.grouping[0].group_value),
      editionNonce: asset.supply.edition_nonce,
      primarySaleHappened: asset.royalty.primary_sale_happened,
      isMutable: asset.mutable,
    };

    const tx = new anchor.web3.Transaction();
    const ix = await program.methods
      .verify(
        {
          root,
          dataHash,
          creatorHash,
          nonce,
          index,
        },
        message
      )
      .accounts({
        leafOwner: new PublicKey(asset.ownership.owner),
        leafDelegate: new PublicKey(asset.ownership.owner),
        merkleTree: treeAddress,
        compressionProgram: SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
      })
      .remainingAccounts(proofPathAsAccounts)
      .instruction();
    tx.add(ix);
    const sx = await program.provider.sendAndConfirm(tx, [payer], {
      skipPreflight: true,
    });

    // This fails due to incorrect owner
    // const sx = await program.provider.sendAndConfirm(tx, [payer], {
    //   skipPreflight: true,
    // });

    console.log(`   Tx Signature: ${sx}`);
  });
});
