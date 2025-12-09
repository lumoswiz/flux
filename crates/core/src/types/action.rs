use alloy::primitives::{Address, B256, Bytes};

use super::primitives::{BidId, BlockNumber, CurrencyAmount, Price, TokenAmount};

pub struct SubmitBidInput {
    pub max_price: Price,
    pub amount: CurrencyAmount,
    pub owner: Address,
}

pub struct SubmitBidParams {
    pub max_price: Price,
    pub amount: CurrencyAmount,
    pub owner: Address,
    pub prev_tick_price: Price,
    pub hook_data: Bytes,
    pub value: CurrencyAmount,
}

pub struct ExitBidParams {
    pub bid_id: BidId,
}

pub struct ExitPartiallyFilledParams {
    pub bid_id: BidId,
    pub last_fully_filled_checkpoint_block: BlockNumber,
    pub outbid_block: Option<BlockNumber>,
}

pub struct ExitHints {
    pub last_fully_filled_checkpoint_block: BlockNumber,
    pub outbid_block: Option<BlockNumber>,
}

pub struct ClaimParams {
    pub owner: Address,
    pub bid_ids: Vec<BidId>,
}

pub struct SubmitBidResult {
    pub bid_id: BidId,
    pub tx_hash: B256,
}

pub struct ExitResult {
    pub bid_id: BidId,
    pub tokens_filled: TokenAmount,
    pub currency_refunded: CurrencyAmount,
    pub tx_hash: B256,
}

pub struct ClaimResult {
    pub bid_ids: Vec<BidId>,
    pub total_tokens: TokenAmount,
    pub tx_hash: B256,
}
