use alloy::primitives::Address;

use super::primitives::{
    BlockNumber, CurrencyAddr, HookAddr, Price, TickSpacing, TokenAddr, TokenAmount,
};

pub struct AuctionConfig {
    pub address: Address,
    pub start_block: BlockNumber,
    pub end_block: BlockNumber,
    pub claim_block: BlockNumber,
    pub total_supply: TokenAmount,
    pub tick_spacing: TickSpacing,
    pub floor_price: Price,
    pub max_bid_price: Price,
    pub currency: CurrencyAddr,
    pub token: TokenAddr,
    pub validation_hook: HookAddr,
}

impl AuctionConfig {
    pub fn is_valid_price(&self, price: Price) -> bool {
        price > self.floor_price
            && price <= self.max_bid_price
            && price.is_aligned(self.tick_spacing)
    }

    pub fn is_native_currency(&self) -> bool {
        self.currency.is_native()
    }
}
