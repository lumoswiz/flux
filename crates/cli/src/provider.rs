use alloy::primitives::Address;
use alloy::providers::{Provider, ReqwestProvider};
use alloy::transports::http::Http;
use std::sync::Arc;

pub type HttpProvider = Provider<Http<ReqwestProvider>>;

#[derive(Clone)]
pub struct ChainContext {
    pub provider: Arc<HttpProvider>,
    pub chain_id: u64,
}

impl ChainContext {
    pub fn new(rpc_url: &str, chain_id: u64) -> eyre::Result<Self> {
        let transport = Http::new(reqwest::Client::new(), rpc_url.parse()?);
        let provider = Provider::new(transport);
        Ok(Self {
            provider: Arc::new(provider),
            chain_id,
        })
    }
}
