elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::{
    common_storage::{BrandId, BrandInfo, MintPrice, TimePeriod},
    nft_attributes_builder::{CollectionHash, Tag},
};

const NFT_ISSUE_COST: u64 = 50_000_000_000_000_000; // 0.05 EGLD
const ROYALTIES_MAX: u32 = 10_000; // 100%

const MAX_BRAND_ID_LEN: usize = 50;
pub static INVALID_BRAND_ID_ERR_MSG: &[u8] = b"Invalid Brand ID";

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode)]
pub struct TempCallbackTierInfo<M: ManagedTypeApi> {
    pub total_nfts: usize,
    pub mint_price: MintPrice<M>,
}

#[derive(TopEncode, TopDecode)]
pub struct TempCallbackStorageInfo<M: ManagedTypeApi> {
    pub brand_info: BrandInfo<M>,
    pub tags: ManagedVec<M, Tag<M>>,
    pub tier_info: TempCallbackTierInfo<M>
}

#[elrond_wasm::module]
pub trait BrandCreationModule:
    crate::admin_whitelist::AdminWhitelistModule
    + crate::common_storage::CommonStorageModule
    + crate::nft_attributes_builder::NftAttributesBuilderModule
    + crate::nft_tier::NftTierModule
    + crate::events::EventsModule
{
    #[payable("EGLD")]
    #[endpoint(issueTokenForBrand)]
    fn issue_token_for_brand(
        &self,
        collection_hash: CollectionHash<Self::Api>,
        brand_id: BrandId<Self::Api>,
        media_type: ManagedBuffer,
        royalties: BigUint,
        mint_start_timestamp: u64,
        mint_end_timestamp: u64,
        mint_price_token_id: EgldOrEsdtTokenIdentifier,
        token_display_name: ManagedBuffer,
        token_ticker: ManagedBuffer,
        whitelist_expire_timestamp: u64,
        tags: ManagedVec<Tag<Self::Api>>,
        total_nfts: usize,
        mint_price_token_amount: BigUint,
    ) {
        self.require_caller_is_admin();

        let id_len = brand_id.len();
        require!(
            id_len > 0 && id_len <= MAX_BRAND_ID_LEN,
            INVALID_BRAND_ID_ERR_MSG
        );

        let payment_amount = self.call_value().egld_value();
        require!(
            payment_amount == NFT_ISSUE_COST,
            "Invalid payment amount. Issue costs exactly 0.05 EGLD"
        );

        require!(
            self.is_supported_media_type(&media_type),
            "Invalid media type"
        );
        require!(royalties <= ROYALTIES_MAX, "Royalties cannot be over 100%");
        require!(mint_price_token_id.is_valid(), "Invalid price token");

        let is_new_collection = self
            .registered_collection_hashes()
            .insert(collection_hash.clone());
        require!(is_new_collection, "Collection hash already exists");

        let is_new_brand = self.registered_brands().insert(brand_id.clone());
        require!(is_new_brand, "Brand already exists");

        require!(
            mint_start_timestamp < mint_end_timestamp,
            "Invalid timestamps"
        );
        require!(
            total_nfts > 0,
            "Invalid total nfts"
        );
        require!(
            mint_price_token_amount > 0,
            "Invalid mint price token amount"
        );      

        let brand_info = BrandInfo {
            collection_hash: collection_hash.clone(),
            token_display_name: token_display_name.clone(),
            media_type,
            royalties,
            mint_period: TimePeriod {
                start: mint_start_timestamp,
                end: mint_end_timestamp,
            },
            whitelist_expire_timestamp,
        };

        self.temporary_callback_storage(&brand_id)
            .set(&TempCallbackStorageInfo {
                brand_info,
                tags,
                tier_info: TempCallbackTierInfo {
                    total_nfts: total_nfts,
                    mint_price: MintPrice {
                        token_id: mint_price_token_id.clone(),
                        amount: mint_price_token_amount,
                    },
                },
            });

        self.nft_token(&brand_id).issue_and_set_all_roles(
            EsdtTokenType::NonFungible,
            payment_amount,
            token_display_name,
            token_ticker,
            0,
            Some(self.callbacks().issue_callback(collection_hash, brand_id)),
        );
    }

    #[callback]
    fn issue_callback(
        &self,
        collection_hash: CollectionHash<Self::Api>,
        brand_id: BrandId<Self::Api>,
        #[call_result] result: ManagedAsyncCallResult<TokenIdentifier>,
    ) {
        match result {
            ManagedAsyncCallResult::Ok(token_id) => {
                let cb_info: TempCallbackStorageInfo<Self::Api> =
                    self.temporary_callback_storage(&brand_id).get();

                self.nft_token(&brand_id).set_token_id(&token_id);
                self.brand_info(&brand_id).set(&cb_info.brand_info);                

                self.available_ids(&brand_id)
                        .set_initial_len(cb_info.tier_info.total_nfts);
                self.total_nfts(&brand_id)
                    .set(cb_info.tier_info.total_nfts);

                self.price_for_tier(&brand_id)
                    .set(&cb_info.tier_info.mint_price);

                if !cb_info.tags.is_empty() {
                    self.tags_for_brand(&brand_id).set(&cb_info.tags);
                }

                self.brand_created_event(&brand_id, &token_id);
            }
            ManagedAsyncCallResult::Err(_) => {
                let _ = self.registered_brands().swap_remove(&brand_id);
                let _ = self
                    .registered_collection_hashes()
                    .swap_remove(&collection_hash);
            }
        }

        self.temporary_callback_storage(&brand_id).clear();
    }

    #[endpoint(setMintToken)]
    fn set_mint_token(
        &self,
        brand_id: BrandId<Self::Api>,
        mint_price_token_id: EgldOrEsdtTokenIdentifier,
        mint_price_token_amount: BigUint,
    ) {
        self.require_caller_is_admin();

        self.price_for_tier(&brand_id)
                    .set(MintPrice {
                        token_id: mint_price_token_id.clone(),
                        amount: mint_price_token_amount,
                    });
    }    

    #[endpoint(addToWhitelist)]
    fn add_to_whitelist(
        &self,
        brand_id: BrandId<Self::Api>,
        users: MultiValueEncoded<ManagedAddress>,
    ) {
        self.require_caller_is_admin();

        let mut mapper = self.mint_whitelist(&brand_id);
        for user in users {
            let _ = mapper.insert(user);
        }
    }

    #[endpoint(removeFromWhitelist)]
    fn remove_from_whitelist(
        &self,
        brand_id: BrandId<Self::Api>,
        users: MultiValueEncoded<ManagedAddress>,
    ) {
        self.require_caller_is_admin();

        let mut mapper = self.mint_whitelist(&brand_id);
        for user in users {
            let _ = mapper.swap_remove(&user);
        }
    }

    #[endpoint(setMintWhitelistExpireTimestamp)]
    fn set_mint_whitelist_expire_timestamp(&self, brand_id: BrandId<Self::Api>, timestamp: u64) {
        self.require_caller_is_admin();

        self.brand_info(&brand_id)
            .update(|info| info.whitelist_expire_timestamp = timestamp);
    }

    #[storage_mapper("temporaryCallbackStorage")]
    fn temporary_callback_storage(
        &self,
        brand_id: &BrandId<Self::Api>,
    ) -> SingleValueMapper<TempCallbackStorageInfo<Self::Api>>;
}
