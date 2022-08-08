pub mod constants;
pub mod nft_collection_setup;
pub mod reward_handler_setup;

use constants::*;
use elrond_wasm::types::{EsdtTokenPayment, ManagedVec, ManagedAddress};
use elrond_wasm_debug::{managed_address, managed_biguint, rust_biguint, DebugApi};
use reward_handler::views::ViewsModule;
use reward_handler::{structs::RewardEntry};
use reward_handler_setup::*;

#[test]
fn setup_test() {
    let _ = DutyRewardHandlerSetup::new(nft_collection::contract_obj, reward_handler::contract_obj);
}

#[test]
fn claim_payments_test() {
    let mut rh_setup =
        DutyRewardHandlerSetup::new(nft_collection::contract_obj, reward_handler::contract_obj);

    rh_setup.b_mock.set_block_epoch(5);
    rh_setup.call_claim_payments().assert_ok();

    // check balance
    rh_setup.b_mock.check_esdt_balance(
        rh_setup.rh_wrapper.address_ref(),
        ROYALTIES_TOKEN_ID,
        &rust_biguint!(ROYALTIES_TOKEN_BALANCE),
    );
    rh_setup.b_mock.check_egld_balance(
        rh_setup.rh_wrapper.address_ref(),
        &rust_biguint!(MINT_PAYMENTS_BALANCE),
    );

    // check internal storage
    rh_setup
        .b_mock
        .execute_query(&rh_setup.rh_wrapper, |sc| {
            assert_eq!(sc.known_tokens().len(), 2);
            assert!(sc
                .known_tokens()
                .contains(&managed_token_id!(ROYALTIES_TOKEN_ID)));
            assert!(sc
                .known_tokens()
                .contains(&managed_token_id!(EGLD_TOKEN_ID)));

            assert_eq!(
                sc.balance_for_token(&managed_token_id!(ROYALTIES_TOKEN_ID))
                    .get(),
                managed_biguint!(ROYALTIES_TOKEN_BALANCE)
            );
            assert_eq!(
                sc.balance_for_token(&managed_token_id!(EGLD_TOKEN_ID))
                    .get(),
                managed_biguint!(MINT_PAYMENTS_BALANCE)
            );
        })
        .assert_ok();

    // try claim again same epoch
    rh_setup
        .call_claim_payments()
        .assert_user_error("Already claimed this epoch");

    // claim after one epoch ok
    rh_setup.b_mock.set_block_epoch(6);
    rh_setup.call_claim_payments().assert_ok();
}

#[test]
fn create_new_reward_entry_test() {
    let mut rh_setup =
        DutyRewardHandlerSetup::new(nft_collection::contract_obj, reward_handler::contract_obj);

    rh_setup.b_mock.set_block_epoch(5);
    rh_setup.call_claim_payments().assert_ok();
    rh_setup.call_create_new_reward_entry().assert_ok();

    // check storage
    let first_sh = rh_setup.first_shareholder_address.clone();
    let second_sh = rh_setup.second_shareholder_address.clone();
    rh_setup
        .b_mock
        .execute_query(&rh_setup.rh_wrapper, |sc| {
            assert_eq!(sc.last_entry_id().get(), 1);
            assert_eq!(
                sc.claimable_tokens_for_reward_entry(1, ManagedAddress::from_address(&first_sh)).get(),
                RewardEntry::<DebugApi> {
                    egld_amount: managed_biguint!(MINT_PAYMENTS_BALANCE / 2),
                    esdt_payments: ManagedVec::from_single_item(EsdtTokenPayment::<DebugApi>::new(
                        managed_token_id!(ROYALTIES_TOKEN_ID).unwrap_esdt(),
                        0,
                        managed_biguint!(ROYALTIES_TOKEN_BALANCE / 2)
                    ))
                }
            );            

            // check list was copied properly
            assert_eq!(sc.claim_whitelist_for_entry(1).len(), 2);
            assert!(sc
                .claim_whitelist_for_entry(1)
                .contains(&managed_address!(&first_sh)));
            assert!(sc
                .claim_whitelist_for_entry(1)
                .contains(&managed_address!(&second_sh)));
        })
        .assert_ok();

    // try create reward entry again same epoch
    rh_setup
        .call_create_new_reward_entry()
        .assert_user_error("Already created reward entry for this epoch");

    // try create reward entry without claim first
    rh_setup.b_mock.set_block_epoch(10);
    rh_setup
        .call_create_new_reward_entry()
        .assert_user_error("Must claim rewards for this epoch first");
}

#[test]
fn claim_rewards_test() {
    let mut rh_setup =
        DutyRewardHandlerSetup::new(nft_collection::contract_obj, reward_handler::contract_obj);

    rh_setup.b_mock.set_block_epoch(5);
    rh_setup.call_claim_payments().assert_ok();
    rh_setup.call_create_new_reward_entry().assert_ok();

    let first_sh = rh_setup.first_shareholder_address.clone();
    let second_sh = rh_setup.second_shareholder_address.clone();

    rh_setup.call_claim_rewards(&first_sh, &[1]).assert_ok();

    // check balance
    rh_setup.b_mock.check_esdt_balance(
        &first_sh,
        ROYALTIES_TOKEN_ID,
        &rust_biguint!(ROYALTIES_TOKEN_BALANCE / 2),
    );
    rh_setup
        .b_mock
        .check_egld_balance(&first_sh, &rust_biguint!(MINT_PAYMENTS_BALANCE / 2));

    // check storage
    rh_setup
        .b_mock
        .execute_query(&rh_setup.rh_wrapper, |sc| {
            // check list was updated
            assert_eq!(sc.claim_whitelist_for_entry(1).len(), 1);
            assert!(sc
                .claim_whitelist_for_entry(1)
                .contains(&managed_address!(&second_sh)));
        })
        .assert_ok();

    // first shareholder try claim again
    rh_setup.call_claim_rewards(&first_sh, &[1]).assert_ok();

    // check balance - first shareholder received nothing
    rh_setup.b_mock.check_esdt_balance(
        &first_sh,
        ROYALTIES_TOKEN_ID,
        &rust_biguint!(ROYALTIES_TOKEN_BALANCE / 2),
    );
    rh_setup
        .b_mock
        .check_egld_balance(&first_sh, &rust_biguint!(MINT_PAYMENTS_BALANCE / 2));

    // second shareholder claim
    rh_setup.call_claim_rewards(&second_sh, &[1]).assert_ok();

    // check balances
    rh_setup.b_mock.check_esdt_balance(
        &second_sh,
        ROYALTIES_TOKEN_ID,
        &rust_biguint!(ROYALTIES_TOKEN_BALANCE / 2),
    );
    rh_setup
        .b_mock
        .check_egld_balance(&second_sh, &rust_biguint!(MINT_PAYMENTS_BALANCE / 2));

    // check storage entry was cleared after everyone claimed
    rh_setup
        .b_mock
        .execute_query(&rh_setup.rh_wrapper, |sc| {
            assert_eq!(sc.claim_whitelist_for_entry(1).len(), 0);
            assert!(sc.claimable_tokens_for_reward_entry(1, ManagedAddress::from_address(&first_sh)).is_empty())
        })
        .assert_ok();
}
