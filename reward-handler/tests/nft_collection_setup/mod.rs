use super::constants::*;
use elrond_wasm::types::{Address, EsdtLocalRole};
use elrond_wasm_debug::{
    managed_address, managed_biguint, managed_buffer, rust_biguint,
    testing_framework::{BlockchainStateWrapper, ContractObjWrapper},
    tx_mock::TxResult,
    DebugApi,
};
use nft_collection::structs::COLLECTION_HASH_LEN;
use nft_collection::storage::StorageModule;
use nft_collection::DutyNftMinter;

#[macro_export]
macro_rules! managed_token_id {
    ($bytes:expr) => {{
        if $bytes == elrond_wasm::types::EgldOrEsdtTokenIdentifier::<DebugApi>::EGLD_REPRESENTATION
        {
            elrond_wasm::types::EgldOrEsdtTokenIdentifier::egld()
        } else {
            let ___token_id___ = elrond_wasm::types::TokenIdentifier::from_esdt_bytes($bytes);
            elrond_wasm::types::EgldOrEsdtTokenIdentifier::esdt(___token_id___)
        }
    }};
}

pub struct DutyNftMinterSetup<DutyNftMinterObjBuilder>
where
    DutyNftMinterObjBuilder: 'static + Copy + Fn() -> nft_collection::ContractObj<DebugApi>,
{
    pub b_mock: BlockchainStateWrapper,
    pub owner_address: Address,
    pub first_user_address: Address,
    pub second_user_address: Address,
    pub nm_wrapper: ContractObjWrapper<nft_collection::ContractObj<DebugApi>, DutyNftMinterObjBuilder>,
}

impl<DutyNftMinterObjBuilder> DutyNftMinterSetup<DutyNftMinterObjBuilder>
where
    DutyNftMinterObjBuilder: 'static + Copy + Fn() -> nft_collection::ContractObj<DebugApi>,
{
    pub fn new(builder: DutyNftMinterObjBuilder) -> Self {
        let rust_zero = rust_biguint!(0u64);
        let mut b_mock = BlockchainStateWrapper::new();
        let owner_address = b_mock.create_user_account(&rust_biguint!(OWNER_EGLD_BALANCE));
        let first_user_address = b_mock.create_user_account(&rust_biguint!(USER_EGLD_BALANCE));
        let second_user_address = b_mock.create_user_account(&rust_biguint!(USER_EGLD_BALANCE));
        let nm_wrapper =
            b_mock.create_sc_account(&rust_zero, Some(&owner_address), builder, "nft minter path");

        // init ESDT System SC mock
        b_mock.create_sc_account_fixed_address(
            &Address::from(ESDT_CONTRACT_ADDRESS_ARRAY),
            &rust_zero,
            None,
            esdt_contract_mock::contract_obj,
            "ESDT system SC mock path",
        );

        b_mock
            .execute_tx(&owner_address, &nm_wrapper, &rust_zero, |sc| {
                sc.init(
                    managed_address!(&owner_address),
                    managed_address!(&owner_address),
                );
            })
            .assert_ok();

        // simulate royalties balance
        b_mock
            .execute_tx(&owner_address, &nm_wrapper, &rust_zero, |sc| {
                sc.accumulated_royalties().insert(
                    managed_token_id!(ROYALTIES_TOKEN_ID),
                    managed_biguint!(ROYALTIES_TOKEN_BALANCE),
                );
            })
            .assert_ok();

        b_mock.set_esdt_balance(
            nm_wrapper.address_ref(),
            ROYALTIES_TOKEN_ID,
            &rust_biguint!(ROYALTIES_TOKEN_BALANCE),
        );

        // simulate mint payments balance
        b_mock
            .execute_tx(&owner_address, &nm_wrapper, &rust_zero, |sc| {
                sc.accumulated_mint_payments().insert(
                    managed_token_id!(EGLD_TOKEN_ID),
                    managed_biguint!(MINT_PAYMENTS_BALANCE),
                );
            })
            .assert_ok();

        b_mock.set_egld_balance(
            nm_wrapper.address_ref(),
            &rust_biguint!(MINT_PAYMENTS_BALANCE),
        );

        Self {
            b_mock,
            owner_address,
            first_user_address,
            second_user_address,
            nm_wrapper,
        }
    }

    pub fn create_default_collections(&mut self) {
        self.call_create_new_collection(
            FIRST_COLLECTION_HASH,
            FIRST_COLLECTION_ID,
            FIRST_MEDIA_TYPE,
            0,
            FIRST_MINT_START_TIMESTAMP,
            FIRST_MINT_END_TIMESTAMP,
            FIRST_MINT_PRICE_TOKEN_ID,
            FIRST_MINT_PRICE_AMOUNT,
            FIRST_TOKEN_DISPLAY_NAME,
            FIRST_TOKEN_TICKER,
            FIRST_TAGS,
            FIRST_NFT_AMOUNTS,
        )
        .assert_ok();

        self.call_create_new_collection(
            SECOND_COLLECTION_HASH,
            SECOND_COLLECTION_ID,
            SECOND_MEDIA_TYPE,
            0,
            SECOND_MINT_START_TIMESTAMP,
            SECOND_MINT_END_TIMESTAMP,
            SECOND_MINT_PRICE_TOKEN_ID,
            SECOND_MINT_PRICE_AMOUNT,
            SECOND_TOKEN_DISPLAY_NAME,
            SECOND_TOKEN_TICKER,
            SECOND_TAGS,
            SECOND_NFT_AMOUNTS,
        )
        .assert_ok();

        self.b_mock.set_esdt_local_roles(
            self.nm_wrapper.address_ref(),
            FIRST_TOKEN_ID,
            &[EsdtLocalRole::NftCreate][..],
        );
        self.b_mock.set_esdt_local_roles(
            self.nm_wrapper.address_ref(),
            SECOND_TOKEN_ID,
            &[EsdtLocalRole::NftCreate][..],
        );
    }
}

impl<DutyNftMinterObjBuilder> DutyNftMinterSetup<DutyNftMinterObjBuilder>
where
    DutyNftMinterObjBuilder: 'static + Copy + Fn() -> nft_collection::ContractObj<DebugApi>,
{
    pub fn call_create_new_collection(
        &mut self,
        collection_hash: &[u8; COLLECTION_HASH_LEN],
        collection_id: &[u8],
        media_type: &[u8],
        royalties: u64,
        mint_start_timestamp: u64,
        mint_end_timestamp: u64,
        mint_price_token_id: &[u8],
        mint_price_amount: u64,
        token_display_name: &[u8],
        token_ticker: &[u8],
        tags: &[u8],
        total_nfts: usize,
    ) -> TxResult {
        self.b_mock.execute_tx(
            &self.owner_address,
            &self.nm_wrapper,
            &rust_biguint!(ISSUE_COST),
            |sc| {

                sc.add_collection(
                    collection_hash.into(),
                    managed_buffer!(collection_id),
                    managed_buffer!(media_type),
                    managed_biguint!(royalties),
                    mint_start_timestamp,
                    mint_end_timestamp,
                    managed_token_id!(mint_price_token_id),
                    managed_buffer!(token_display_name),
                    managed_buffer!(token_ticker),
                    0,
                    managed_buffer!(tags),
                    total_nfts,
                    managed_biguint!(mint_price_amount)
                );
            },
        )
    }
}
