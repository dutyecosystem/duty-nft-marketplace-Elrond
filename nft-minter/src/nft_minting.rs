elrond_wasm::imports!();

use crate::{
    brand_creation::{INVALID_BRAND_ID_ERR_MSG},
    common_storage::{BrandId, BrandInfo, PaymentsVec},
};

const NFT_AMOUNT: u32 = 1;

#[elrond_wasm::module]
pub trait NftMintingModule:
    crate::common_storage::CommonStorageModule
    + crate::nft_tier::NftTierModule
    + crate::royalties::RoyaltiesModule
    + crate::admin_whitelist::AdminWhitelistModule
    + crate::nft_attributes_builder::NftAttributesBuilderModule
    + crate::events::EventsModule
{
    #[payable("*")]
    #[endpoint(mintNft)]
    fn mint_nft(
        &self,
        brand_id: BrandId<Self::Api>,
        opt_nfts_to_buy: OptionalValue<usize>,
    ) -> PaymentsVec<Self::Api> {
        require!(
            self.registered_brands().contains(&brand_id),
            INVALID_BRAND_ID_ERR_MSG
        );

        let nfts_to_buy = match opt_nfts_to_buy {
            OptionalValue::Some(val) => {
                if val == 0 {
                    return PaymentsVec::new();
                }
                val
            }
            OptionalValue::None => NFT_AMOUNT as usize,
        };

        let price_for_tier = self.price_for_tier(&brand_id).get();
        let payment = self.call_value().egld_or_single_esdt();
        let total_required_amount = &price_for_tier.amount * (nfts_to_buy as u32);
        require!(
            payment.token_identifier == price_for_tier.token_id
                && payment.amount == total_required_amount,
            "Invalid payment"
        );

        let brand_info: BrandInfo<Self::Api> = self.brand_info(&brand_id).get();
        let current_timestamp = self.blockchain().get_block_timestamp();
        require!(
            current_timestamp >= brand_info.mint_period.start,
            "May not mint yet"
        );
        require!(
            current_timestamp < brand_info.mint_period.end,
            "May not mint after deadline"
        );

        let caller = self.blockchain().get_caller();
        if current_timestamp < brand_info.whitelist_expire_timestamp {
            require!(
                self.mint_whitelist(&brand_id).contains(&caller),
                "Not in whitelist"
            );
        }

        self.add_mint_payment(payment.token_identifier, payment.amount);

        let output_payments =
            self._mint_and_send_random_nft(&caller, &brand_id, &brand_info, nfts_to_buy);

        self.nft_bought_event(&caller, &brand_id, nfts_to_buy);

        output_payments
    }

    #[endpoint(giveawayNfts)]
    fn giveaway_nfts(
        &self,
        brand_id: BrandId<Self::Api>,
        dest_amount_pairs: MultiValueEncoded<MultiValue2<ManagedAddress, usize>>,
    ) {
        self.require_caller_is_admin();

        require!(
            self.registered_brands().contains(&brand_id),
            INVALID_BRAND_ID_ERR_MSG
        );        

        let brand_info = self.brand_info(&brand_id).get();
        let mut total = 0;
        for pair in dest_amount_pairs {
            let (dest_address, nfts_to_send) = pair.into_tuple();
            if nfts_to_send > 0 {
                let _ = self._mint_and_send_random_nft(
                    &dest_address,
                    &brand_id,
                    &brand_info,
                    nfts_to_send,
                );
                total += nfts_to_send;
            }
        }

        self.nft_giveaway_event(&brand_id, total);
    }

    #[endpoint(claimNfts)]
    fn claim_nfts(
        &self,
        brand_id: BrandId<Self::Api>,
        claim_amount: usize,
    ) {
        self.require_caller_is_admin();

        require!(
            self.registered_brands().contains(&brand_id),
            INVALID_BRAND_ID_ERR_MSG
        );        

        require!(
            claim_amount > 0,
            "Claim amount must be greater than 0"
        );

        let brand_info = self.brand_info(&brand_id).get();
        let _ = self._mint_and_send_random_nft(
            &self.blockchain().get_caller(),
            &brand_id,
            &brand_info,
            claim_amount,
        );

        self.nft_claimed_event(&brand_id, claim_amount);
    }

    #[endpoint(claimNftsByIds)]
    fn claim_nfts_by_ids(
        &self,
        brand_id: BrandId<Self::Api>,
        nft_ids: MultiValueEncoded<usize>,
    ) {
        self.require_caller_is_admin();

        require!(
            self.registered_brands().contains(&brand_id),
            INVALID_BRAND_ID_ERR_MSG
        );        

        let brand_info = self.brand_info(&brand_id).get();

        let nft_token_id = self.nft_token(&brand_id).get_token_id();
        let mut nft_output_payments = ManagedVec::new();
        let mut nfts_count = 0;
        
        for nft_id in nft_ids {
            let _ = self.verify_nft_id(&brand_id, nft_id);
            let nft_uri = self.build_nft_main_file_uri(
                &brand_info.collection_hash,
                nft_id,
                &brand_info.media_type,
            );
            let nft_json = self.build_nft_json_file_uri(&brand_info.collection_hash, nft_id);
            let collection_json = self.build_collection_json_file_uri(&brand_info.collection_hash);

            let mut uris = ManagedVec::new();
            uris.push(nft_uri);
            uris.push(nft_json);
            uris.push(collection_json);

            let attributes =
                self.build_nft_attributes(&brand_info.collection_hash, &brand_id, nft_id);
            let nft_amount = BigUint::from(NFT_AMOUNT);
            let nft_nonce = self.send().esdt_nft_create(
                &nft_token_id,
                &nft_amount,
                &brand_info.token_display_name,
                &brand_info.royalties,
                &ManagedBuffer::new(),
                &attributes,
                &uris,
            );

            nft_output_payments.push(EsdtTokenPayment::new(
                nft_token_id.clone(),
                nft_nonce,
                nft_amount,
            ));
            nfts_count += 1;
        }

        require!(
            nfts_count <= self.available_ids(&brand_id).len(),
            "Not enough NFTs available"
        );

        self.send().direct_multi(&self.blockchain().get_caller(), &nft_output_payments);

        self.nft_claimed_event(&brand_id, nfts_count);
    }

    fn _mint_and_send_random_nft(
        &self,
        to: &ManagedAddress,
        brand_id: &BrandId<Self::Api>,
        brand_info: &BrandInfo<Self::Api>,
        nfts_to_send: usize,
    ) -> PaymentsVec<Self::Api> {
        require!(
            !self.blockchain().is_smart_contract(to),
            "Only user accounts are allowed to mint"
        );

        let total_available_nfts = self.available_ids(brand_id).len();
        require!(
            nfts_to_send <= total_available_nfts,
            "Not enough NFTs available"
        );

        let nft_token_id = self.nft_token(brand_id).get_token_id();
        let mut nft_output_payments = ManagedVec::new();
        for _ in 0..nfts_to_send {
            let nft_id = self.get_next_random_id(brand_id);
            let nft_uri = self.build_nft_main_file_uri(
                &brand_info.collection_hash,
                nft_id,
                &brand_info.media_type,
            );
            let nft_json = self.build_nft_json_file_uri(&brand_info.collection_hash, nft_id);
            let collection_json = self.build_collection_json_file_uri(&brand_info.collection_hash);

            let mut uris = ManagedVec::new();
            uris.push(nft_uri);
            uris.push(nft_json);
            uris.push(collection_json);

            let attributes =
                self.build_nft_attributes(&brand_info.collection_hash, brand_id, nft_id);
            let nft_amount = BigUint::from(NFT_AMOUNT);
            let nft_nonce = self.send().esdt_nft_create(
                &nft_token_id,
                &nft_amount,
                &brand_info.token_display_name,
                &brand_info.royalties,
                &ManagedBuffer::new(),
                &attributes,
                &uris,
            );

            nft_output_payments.push(EsdtTokenPayment::new(
                nft_token_id.clone(),
                nft_nonce,
                nft_amount,
            ));
        }

        self.send().direct_multi(to, &nft_output_payments);

        nft_output_payments
    }
}
