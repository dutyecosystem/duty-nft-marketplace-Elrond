elrond_wasm::imports!();

// use crate::structs::{Stake};

#[elrond_wasm::module]
pub trait PrivateFunctionsModule:
    crate::views::ViewsModule
    + crate::events::EventsModule
{
    fn get_nft_info(&self, nft_type: &TokenIdentifier, nft_nonce: u64) -> EsdtTokenData<Self::Api> {
        self.blockchain().get_esdt_token_data(
            &self.blockchain().get_sc_address(),
            nft_type,
            nft_nonce
        )
    }
}

