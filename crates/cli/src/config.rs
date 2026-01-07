use std::path::Path;
use std::str::FromStr;

use alloy::primitives::U256;
use eyre::{Result, eyre};
use serde::Deserialize;

use flux_core::{CurrencyAmount, Price};

#[derive(Debug, Deserialize)]
pub struct BidsConfig {
    pub bids: Vec<BidConfig>,
}

#[derive(Debug, Deserialize)]
pub struct BidConfig {
    /// Maximum price in wei
    pub max_price: String,
    /// Amount in wei
    pub amount: String,
}

impl BidConfig {
    pub fn max_price(&self) -> Result<Price> {
        let value = U256::from_str(&self.max_price)
            .map_err(|e| eyre!("Invalid max_price '{}': {}", self.max_price, e))?;
        Ok(Price::new(value))
    }

    pub fn amount(&self) -> Result<CurrencyAmount> {
        let value = U256::from_str(&self.amount)
            .map_err(|e| eyre!("Invalid amount '{}': {}", self.amount, e))?;
        Ok(CurrencyAmount::new(value))
    }
}

pub fn load_bids_config(path: impl AsRef<Path>) -> Result<BidsConfig> {
    let content = std::fs::read_to_string(path.as_ref()).map_err(|e| {
        eyre!(
            "Failed to read config file '{}': {}",
            path.as_ref().display(),
            e
        )
    })?;

    let config: BidsConfig =
        toml::from_str(&content).map_err(|e| eyre!("Failed to parse config file: {}", e))?;

    if config.bids.is_empty() {
        return Err(eyre!("Config file must contain at least one bid"));
    }

    Ok(config)
}

pub fn get_rpc_url() -> Result<String> {
    std::env::var("RPC_URL").map_err(|_| eyre!("RPC_URL environment variable not set"))
}
