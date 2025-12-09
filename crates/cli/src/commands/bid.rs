// src/commands/bid.rs
use crate::domain::price::q96_from_ratio;
use crate::provider::ChainContext;
use alloy::primitives::{Address, U256};
use anyhow::Result;
use flux_abi::IContinuousClearingAuction;

pub struct BidArgs {
    pub auction: Address,
    pub amount_wei: U256,     // amount in currency wei
    pub max_price_human: f64, // currency_per_token
    pub token_decimals: u8,
    pub currency_decimals: u8,
    pub owner: Address,
    pub prev_tick_price: Option<U256>,
    pub hook_data: Vec<u8>,
}

pub async fn submit_bid(ctx: &ChainContext, args: BidArgs) -> Result<U256> {
    let max_price_q96 = q96_from_ratio(
        args.max_price_human,
        args.token_decimals,
        args.currency_decimals,
    )?;

    let auction = IContinuousClearingAuction::new(args.auction, ctx.provider.clone());

    // Alloy-style call builder; you’ll need to adapt to exact API version:
    let call = if let Some(prev) = args.prev_tick_price {
        auction.submitBid(
            max_price_q96,
            args.amount_wei.to::<u128>() as u128,
            args.owner,
            prev,
            args.hook_data.clone(),
        )
    } else {
        auction.submitBid(
            max_price_q96,
            args.amount_wei.to::<u128>() as u128,
            args.owner,
            Vec::<u8>::new(),
        )
    };

    // TODO: attach signer and send transaction.
    // let tx = call.value(if is_native_currency { args.amount_wei } else { U256::ZERO });
    // let pending = ctx.signer.send_transaction(tx).await?;
    // let receipt = pending.get_receipt().await?;

    // TODO: parse BidSubmitted event to get bidId.
    // For now, just return 0 as a placeholder.
    Ok(U256::from(0u64))
}
