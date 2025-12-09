use alloy::primitives::{Address, U256};
use alloy::providers::Provider;
use flux_abi::{IContinuousClearingAuction, IERC20Minimal};

use crate::{
    error::{ConfigError, Error, StateError},
    types::{
        checkpoint::Checkpoint,
        config::AuctionConfig,
        primitives::{
            BlockNumber, CurrencyAddr, CurrencyAmount, HookAddr, Mps, Price, TickSpacing,
            TokenAddr, TokenAmount,
        },
        state::{AuctionState, GraduationStatus, TokenDepositStatus},
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
