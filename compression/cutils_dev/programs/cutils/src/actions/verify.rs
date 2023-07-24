use std::str::FromStr;

use crate::*;
use mpl_bubblegum::state::leaf_schema::LeafSchema;
use mpl_bubblegum::state::metaplex_adapter::{MetadataArgs, TokenProgramVersion, TokenStandard};
use mpl_bubblegum::state::metaplex_adapter::{Creator as MetaplexCreator, Collection as MetaplexCollection};
use mpl_bubblegum::hash_metadata;
use mpl_bubblegum::utils::get_asset_id;
use spl_account_compression::program::SplAccountCompression;

#[derive(Accounts)]
#[instruction(params: VerifyParams, message: Metadata)]
pub struct Verify<'info> {
    pub leaf_owner: Signer<'info>,

    /// CHECK: This account is neither written to nor read from.
    pub leaf_delegate: AccountInfo<'info>,

    /// CHECK: unsafe
    pub merkle_tree: UncheckedAccount<'info>,

    pub compression_program: Program<'info, SplAccountCompression>,
}

#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct VerifyParams {
    root: [u8; 32],
    data_hash: [u8; 32],
    creator_hash: [u8; 32],
    nonce: u64,
    index: u32,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct Metadata {
    name: String,
    symbol: String,
    uri: String,
    collection: Collection,
    seller_fee_basis_points: u16,
    primary_sale_happened: bool,
    is_mutable: bool,
    edition_nonce: Option<u8>,
    creators: Vec<Creator>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct Collection {
    verified: bool,
    key: Pubkey,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct Creator {
    address: Pubkey,
    verified: bool,
    share: u8,
}

impl Verify<'_> {
    pub fn validate(
        &self,
        _ctx: &Context<Self>,
        _params: &VerifyParams,
    ) -> Result<()> {
        Ok(())
    }

    pub fn actuate<'info>(ctx: Context<'_, '_, '_, 'info, Verify<'info>>, params: &VerifyParams, message: &Metadata) -> Result<()> {
        let asset_id = get_asset_id(&ctx.accounts.merkle_tree.key(), params.nonce);

        let mut creators = vec![];
        for creator in message.creators.clone().iter() {
            creators.push(MetaplexCreator {
                address: creator.address.clone(),
                verified: creator.verified,
                share: creator.share,
            });
        }
        let metadata = MetadataArgs {
            name: message.name.clone(),
            symbol: message.symbol.clone(),
            uri: message.uri.clone(),
            seller_fee_basis_points: message.seller_fee_basis_points,
            creators,
            primary_sale_happened: message.primary_sale_happened,
            is_mutable: message.is_mutable,
            edition_nonce: message.edition_nonce,
            collection: Some(MetaplexCollection {
                verified: message.collection.verified,
                key: message.collection.key,
            }),
            uses: None,
            token_program_version: TokenProgramVersion::Original,
            token_standard: Some(TokenStandard::NonFungible),
        };
        let incoming_data_hash = hash_metadata(&metadata).unwrap();
        msg!("incoming_data_hash: {:?}", incoming_data_hash);
        msg!("params.data_hash: {:?}", params.data_hash);
        assert_eq!(incoming_data_hash, params.data_hash, "Data hash does not match");
        
        let leaf = LeafSchema::new_v0(
            asset_id,
            ctx.accounts.leaf_owner.key(),
            ctx.accounts.leaf_delegate.key(),
            params.nonce,
            params.data_hash,
            params.creator_hash,
        );

        let cpi_ctx = CpiContext::new(
            ctx.accounts.compression_program.to_account_info(),
            spl_account_compression::cpi::accounts::VerifyLeaf {
                merkle_tree: ctx.accounts.merkle_tree.to_account_info(),
            },
        ).with_remaining_accounts(ctx.remaining_accounts.to_vec());

        spl_account_compression::cpi::verify_leaf(
            cpi_ctx,
            params.root,
            leaf.to_node(),
            params.index,
        )?;

        Ok(())
    }
}

