use alloy::{
    primitives::{Address, U256},
    providers::{Provider, ProviderBuilder},
};
use eyre::Result;

use crate::domain::{AuctionInfo, BidInfo, BidStatus, ExtraAuctionInfo};
use flux_abi::{
    IAuctionStateLens, IBidStorage, IContinuousClearingAuction, IStepStorage, ITokenCurrencyStorage,
};

#[derive(Debug, Clone)]
pub struct StatusOutput {
    pub auction: AuctionInfo,
    pub bid: BidInfo,
    pub bid_status: BidStatus,
    pub current_block: u64,
}

pub async fn status(
    rpc_url: &str,
    auction_addr: Address,
    lens_addr: Address,
    bid_id: U256,
) -> Result<StatusOutput> {
    // 1. Build provider
    let provider = ProviderBuilder::new().connect(rpc_url).await?;

    // 2. Instantiate contracts / interfaces
    let auction = IContinuousClearingAuction::new(auction_addr, provider.clone());
    let lens = IAuctionStateLens::new(lens_addr, provider.clone());

    // Interfaces for inherited methods:
    let step_storage = IStepStorage::new(auction_addr, provider.clone());
    let token_currency = ITokenCurrencyStorage::new(auction_addr, provider.clone());
    let bid_storage = IBidStorage::new(auction_addr, provider.clone());

    // 3. Get latest auction state via lens (this also checkpoints under the hood)
    let state = lens.state(auction_addr).call().await?;

    // 4. Get extra info not in AuctionState from the other interfaces
    let start_block = step_storage.startBlock().call().await?;
    let end_block = step_storage.endBlock().call().await?;
    let claim_block = auction.claimBlock().call().await?; // this one *is* on IContinuousClearingAuction

    let token = token_currency.token().call().await?;
    let currency_addr: Address = token_currency.currency().call().await?;

    let extra = ExtraAuctionInfo {
        start_block,
        end_block,
        claim_block,
        token: token.into(),
        currency: currency_addr,
    };

    let auction_info = AuctionInfo::from_lens_state(auction_addr, state, extra);

    // 5. Fetch bid and map to domain
    let abi_bid = bid_storage.bids(bid_id).call().await?;
    let bid_info: BidInfo = (auction_addr, bid_id, abi_bid).into();

    // 6. Get current block and derive bid status
    let current_block = provider.get_block_number().await?;
    let bid_status = bid_info.derive_status(current_block, &auction_info);

    Ok(StatusOutput {
        auction: auction_info,
        bid: bid_info,
        bid_status,
        current_block,
    })
}
