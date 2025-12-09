use std::sync::Arc;

use alloy::primitives::{Address, U256};
use alloy::providers::Provider;
use flux_abi::{IContinuousClearingAuction, IERC20Minimal};

use crate::{
    error::{ConfigError, Error, StateError},
    hooks::ValidationHook,
    types::{
        bid::Bid,
        checkpoint::Checkpoint,
        config::AuctionConfig,
        primitives::{
            BidId, BlockNumber, CurrencyAddr, CurrencyAmount, HookAddr, Mps, Price, TickSpacing,
            TokenAddr, TokenAmount,
        },
        state::{AuctionState, GraduationStatus, TokenDepositStatus},
    },
};

pub struct AuctionClient<P>
where
    P: Provider + Clone,
{
    provider: P,
    auction: Address,
    owner: Address,
    hook: Arc<dyn ValidationHook>,
    tracked_bids: Vec<BidId>,
    config: AuctionConfig,
}

impl<P> AuctionClient<P>
where
    P: Provider + Clone,
{
    pub async fn new(
        provider: P,
        auction: Address,
        owner: Address,
        hook: impl Into<Arc<dyn ValidationHook>>,
        tracked_bids: Vec<BidId>,
    ) -> Result<Self, Error> {
        let config = Self::fetch_config(&provider, auction).await?;
        Ok(Self {
            provider,
            auction,
            owner,
            hook: hook.into(),
            tracked_bids,
            config,
        })
    }

    pub fn config(&self) -> &AuctionConfig {
        &self.config
    }

    pub fn address(&self) -> Address {
        self.auction
    }

    pub async fn fetch_state(&self) -> Result<AuctionState, Error> {
        let cca = IContinuousClearingAuction::new(self.auction, &self.provider);
        let token = IERC20Minimal::new(self.config.token.as_address(), &self.provider);

        let (
            checkpoint_raw,
            is_graduated,
            currency_raised_raw,
            last_checkpoint_block,
            token_balance,
        ) = self
            .provider
            .multicall()
            .add(cca.latestCheckpoint())
            .add(cca.isGraduated())
            .add(cca.currencyRaised())
            .add(cca.lastCheckpointedBlock())
            .add(token.balanceOf(self.auction))
            .aggregate()
            .await
            .map_err(StateError::from)?;

        let current_block = BlockNumber::new(
            self.provider
                .get_block_number()
                .await
                .map_err(StateError::from)?,
        );

        let checkpoint = Checkpoint {
            block: BlockNumber::new(last_checkpoint_block),
            clearing_price: Price::new(checkpoint_raw.clearingPrice),
            cumulative_mps: Mps::new(checkpoint_raw.cumulativeMps),
            prev_block: BlockNumber::new(checkpoint_raw.prev),
            next_block: BlockNumber::new(checkpoint_raw.next),
        };

        let tokens_received = if U256::from(token_balance) >= self.config.total_supply.as_u256() {
            TokenDepositStatus::Received
        } else {
            TokenDepositStatus::NotReceived
        };

        let phase = AuctionState::compute_phase(&self.config, current_block, tokens_received);

        let graduation = if is_graduated {
            GraduationStatus::Graduated
        } else {
            GraduationStatus::NotGraduated
        };

        let currency_raised = CurrencyAmount::new(currency_raised_raw);

        Ok(AuctionState {
            current_block,
            phase,
            checkpoint,
            graduation,
            tokens_received,
            currency_raised,
        })
    }

    pub async fn fetch_bids(&self, bid_ids: &[BidId]) -> Result<Vec<Bid>, Error> {
        // Might we want to throw here?
        if bid_ids.is_empty() {
            return Ok(Vec::new());
        }

        let cca = IContinuousClearingAuction::new(self.auction, &self.provider);

        if bid_ids.len() == 1 {
            let bid_id = bid_ids[0];
            let bid_return = cca
                .bids(bid_id.as_u256())
                .call()
                .await
                .map_err(StateError::from)?;
            return Ok(vec![Self::decode_bid(bid_id, bid_return)]);
        }

        let mut multicall = self.provider.multicall().dynamic();

        for bid_id in bid_ids {
            multicall = multicall.add_dynamic(cca.bids(bid_id.as_u256()));
        }

        let bid_returns = multicall.aggregate().await.map_err(StateError::from)?;

        let bids = bid_ids
            .iter()
            .zip(bid_returns.into_iter())
            .map(|(bid_id, bid_return)| Self::decode_bid(*bid_id, bid_return))
            .collect();

        Ok(bids)
    }

    fn decode_bid(bid_id: BidId, bid_return: IContinuousClearingAuction::Bid) -> Bid {
        let exited_block = if bid_return.exitedBlock == 0 {
            None
        } else {
            Some(BlockNumber::new(bid_return.exitedBlock))
        };

        Bid {
            id: bid_id,
            owner: bid_return.owner,
            max_price: Price::new(bid_return.maxPrice),
            amount: CurrencyAmount::new(bid_return.amountQ96),
            start_block: BlockNumber::new(bid_return.startBlock),
            start_cumulative_mps: Mps::new(bid_return.startCumulativeMps),
            exited_block,
            tokens_filled: TokenAmount::new(bid_return.tokensFilled),
        }
    }

    pub async fn fetch_config(provider: &P, auction: Address) -> Result<AuctionConfig, Error> {
        let cca = IContinuousClearingAuction::new(auction, provider);

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
            .add(cca.startBlock())
            .add(cca.endBlock())
            .add(cca.claimBlock())
            .add(cca.totalSupply())
            .add(cca.tickSpacing())
            .add(cca.floorPrice())
            .add(cca.MAX_BID_PRICE())
            .add(cca.currency())
            .add(cca.token())
            .add(cca.validationHook())
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
