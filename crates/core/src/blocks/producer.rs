use alloy::{
    consensus::BlockHeader,
    providers::Provider,
    transports::{TransportError, TransportErrorKind},
};
use futures::{Stream, StreamExt, stream::BoxStream};

use crate::{error::BlockStreamError, types::primitives::BlockNumber};

pub type BlockStreamItem = Result<BlockNumber, BlockStreamError>;

pub trait BlockStream: Stream<Item = BlockStreamItem> + Send + Unpin {}

impl<T> BlockStream for T where T: Stream<Item = BlockStreamItem> + Send + Unpin {}

pub type BoxBlockStream = BoxStream<'static, BlockStreamItem>;

#[derive(Clone)]
pub struct BlockProducer<P>
where
    P: Provider + Clone,
{
    provider: P,
}

impl<P> BlockProducer<P>
where
    P: Provider + Clone,
{
    pub fn new(provider: P) -> Self {
        Self { provider }
    }

    pub async fn into_stream(self) -> Result<BoxBlockStream, BlockStreamError> {
        match self.try_subscribe().await {
            Ok(stream) => Ok(stream),
            Err(BlockStreamError::Transport(err)) => match err {
                TransportError::Transport(TransportErrorKind::PubsubUnavailable) => {
                    self.watch().await
                }
                other => Err(BlockStreamError::Transport(other)),
            },
        }
    }

    async fn try_subscribe(&self) -> Result<BoxBlockStream, BlockStreamError> {
        let subscription = self.provider.subscribe_blocks().await?;
        let stream = subscription
            .into_stream()
            .map(|header| Ok(BlockNumber::new(header.number())))
            .boxed();
        Ok(stream)
    }

    async fn watch(&self) -> Result<BoxBlockStream, BlockStreamError> {
        let watcher = self.provider.watch_full_blocks().await?;
        let stream = watcher
            .into_stream()
            .map(|result| {
                result
                    .map_err(BlockStreamError::from)
                    .map(|block| BlockNumber::new(block.header.number()))
            })
            .boxed();
        Ok(stream)
    }
}
