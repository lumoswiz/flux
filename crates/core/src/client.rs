use alloy::primitives::{Address, U256};
use alloy::providers::Provider;
use flux_abi::IContinuousClearingAuction;

use crate::{
    error::{ConfigError, Error},
    types::{
        config::AuctionConfig,
        primitives::{
            BlockNumber, CurrencyAddr, HookAddr, Price, TickSpacing, TokenAddr, TokenAmount,
        },
    },
};

pub struct AuctionClient<P>
where
    P: Provider,
{
    provider: P,
    auction: Address,
    config: AuctionConfig,
}

impl<P> AuctionClient<P>
where
    P: Provider,
{
    pub async fn new(provider: P, auction: Address) -> Result<Self, Error> {
        let config = Self::fetch_config(&provider, auction).await?;
        Ok(Self {
            provider,
            auction,
            config,
        })
    }

    pub fn config(&self) -> &AuctionConfig {
        &self.config
    }

    pub fn address(&self) -> Address {
        self.auction
    }

    pub async fn fetch_config(provider: &P, auction: Address) -> Result<AuctionConfig, Error> {
        let contract = IContinuousClearingAuction::new(auction, provider);

        let (
            start_block,
            end_block,
            claim_block,
            total_supply,
            tick_spacing,
            floor_price,
            max_bid_price,
            currency,
            token,
            validation_hook,
        ) = provider
            .multicall()
            .add(contract.startBlock())
            .add(contract.endBlock())
            .add(contract.claimBlock())
            .add(contract.totalSupply())
            .add(contract.tickSpacing())
            .add(contract.floorPrice())
            .add(contract.MAX_BID_PRICE())
            .add(contract.currency())
            .add(contract.token())
            .add(contract.validationHook())
            .aggregate()
            .await
            .map_err(ConfigError::from)?;

        Ok(AuctionConfig {
            address: auction,
            start_block: BlockNumber::new(start_block),
            end_block: BlockNumber::new(end_block),
            claim_block: BlockNumber::new(claim_block),
            total_supply: TokenAmount::new(U256::from(total_supply)),
            tick_spacing: TickSpacing::new(tick_spacing),
            floor_price: Price::new(floor_price),
            max_bid_price: Price::new(max_bid_price),
            currency: CurrencyAddr::new(currency),
            token: TokenAddr::new(token),
            validation_hook: HookAddr::new(validation_hook),
        })
    }
}
