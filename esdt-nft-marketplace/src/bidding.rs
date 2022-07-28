elrond_wasm::imports!();

use crate::auction::{Auction, AuctionType};

#[elrond_wasm::module]
pub trait BiddingModule:
    crate::auction::AuctionModule
    + crate::events::EventsModule
    + crate::token_distribution::TokenDistributionModule
    + crate::common_util_functions::CommonUtilFunctions
{
    #[payable("*")]
    #[endpoint]
    fn bid(&self, auction_id: u64, nft_type: TokenIdentifier, nft_nonce: u64) {
        let (payment_token, payment_token_nonce, payment_amount) =
            self.call_value().egld_or_single_esdt().into_tuple();
        let mut auction = self.try_get_auction(auction_id);
        let caller = self.blockchain().get_caller();

        self.common_bid_checks(
            &auction,
            &nft_type,
            nft_nonce,
            &payment_token,
            payment_token_nonce,
        );

        require!(
            auction.auction_type != AuctionType::FixedType,
            "Cannot bid on this type of auction"
        );
        require!(auction.current_winner != caller, "Can't outbid yourself");
        require!(
            payment_amount >= auction.min_bid,
            "Bid must be higher than or equal to the min bid"
        );
        require!(
            payment_amount > auction.current_bid,
            "Bid must be higher than the current winning bid"
        );

        if let Some(max_bid) = &auction.max_bid {
            require!(
                &payment_amount <= max_bid,
                "Bid must be less than or equal to the max bid"
            );
        }

        if auction.current_bid > 0 {
            if let Some(max_bid) = &auction.max_bid {
                if &payment_amount < max_bid {
                    require!(
                        (&payment_amount - &auction.current_bid) >= auction.min_bid_diff,
                        "The difference from the last bid must be higher"
                    );
                }
            }
        }

        // refund losing bid
        if auction.current_winner != ManagedAddress::zero() {
            self.transfer_or_save_payment(
                &auction.current_winner,
                &auction.payment_token,
                auction.payment_nonce,
                &auction.current_bid,
            );
        }

        // update auction bid and winner
        auction.current_bid = payment_amount;
        auction.current_winner = caller;
        self.auction_by_id(auction_id).set(&auction);

        self.emit_bid_event(auction_id, auction);
    }

    #[payable("*")]
    #[endpoint(buyNft)]
    fn buy_nft(
        &self,
        auction_id: u64,
        nft_type: TokenIdentifier,
        nft_nonce: u64,
    ) {
        let (payment_token, payment_token_nonce, payment_amount) =
            self.call_value().egld_or_single_esdt().into_tuple();
        let mut auction = self.try_get_auction(auction_id);
        let caller = self.blockchain().get_caller();

        self.common_bid_checks(
            &auction,
            &nft_type,
            nft_nonce,
            &payment_token,
            payment_token_nonce,
        );

        require!(
            auction.auction_type == AuctionType::FixedType,
            "Cannot buy NFT for this type of auction"
        );
 
        require!(
            &auction.min_bid == &payment_amount,
            "Wrong amount paid, must pay equal to the selling price"
        );

        auction.current_winner = caller;
        auction.current_bid = payment_amount;
        self.distribute_tokens_after_auction_end(&auction);

        self.auction_by_id(auction_id).clear();

        self.emit_buy_nft_event(auction_id, auction);
    }

    fn common_bid_checks(
        &self,
        auction: &Auction<Self::Api>,
        nft_type: &TokenIdentifier,
        nft_nonce: u64,
        payment_token: &EgldOrEsdtTokenIdentifier,
        payment_nonce: u64,
    ) {
        let caller = self.blockchain().get_caller();
        let current_time = self.blockchain().get_block_timestamp();

        require!(
            &auction.auctioned_tokens.token_identifier == nft_type
                && auction.auctioned_tokens.token_nonce == nft_nonce,
            "Auction ID does not match the token"
        );
        require!(
            auction.original_owner != caller,
            "Can't bid on your own token"
        );
        require!(
            current_time >= auction.start_time,
            "Auction hasn't started yet"
        );
        require!(current_time < auction.deadline, "Auction ended already");
        require!(
            payment_token == &auction.payment_token && payment_nonce == auction.payment_nonce,
            "Wrong token used as payment"
        );
    }
}
