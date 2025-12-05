use serde::Deserialize;
use std::{
    env, fs,
    path::{Path, PathBuf},
};
use thiserror::Error;

pub const DEFAULT_CONFIG_PATH: &str = "bids.toml";
const ENV_EXAMPLE: &str = include_str!("./.env.example");
const PRIVATE_KEY_ENV: &str = "PRIVATE_KEY";

#[derive(Debug, Deserialize, PartialEq)]
pub struct BidsConfig {
    pub bid: BidConfig,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct BidConfig {
    pub max_bid: f64,
    pub amount: f64,
    pub owner: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Bid {
    pub max_bid: f64,
    pub amount: f64,
    pub owner: String,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct BidOverrides {
    pub max_bid: Option<f64>,
    pub amount: Option<f64>,
    pub owner: Option<String>,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config at {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to parse toml at {path}: {source}")]
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },
}

#[derive(Debug, Error, PartialEq)]
pub enum BidError {
    #[error("missing owner: pass --owner or set {PRIVATE_KEY_ENV}")]
    MissingOwner,
}

pub fn load_config(path: impl AsRef<Path>) -> Result<BidsConfig, ConfigError> {
    let path = path.as_ref();
    let contents = fs::read_to_string(path).map_err(|source| ConfigError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let config: BidsConfig = toml::from_str(&contents).map_err(|source| ConfigError::Parse {
        path: path.to_path_buf(),
        source,
    })?;

    Ok(config)
}

pub fn load_default_config() -> Result<BidsConfig, ConfigError> {
    load_config(DEFAULT_CONFIG_PATH)
}

pub fn resolve_bid(config: &BidsConfig, overrides: BidOverrides) -> Result<Bid, BidError> {
    let max_bid = overrides.max_bid.unwrap_or(config.bid.max_bid);
    let amount = overrides.amount.unwrap_or(config.bid.amount);
    let owner = overrides
        .owner
        .or_else(|| config.bid.owner.clone())
        .or_else(owner_from_env)
        .ok_or(BidError::MissingOwner)?;

    Ok(Bid {
        max_bid,
        amount,
        owner,
    })
}

fn owner_from_env() -> Option<String> {
    env::var(PRIVATE_KEY_ENV)
        .ok()
        .or_else(|| parse_env_example(PRIVATE_KEY_ENV))
}

fn parse_env_example(key: &str) -> Option<String> {
    ENV_EXAMPLE
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .find_map(|line| {
            let mut parts = line.splitn(2, '=');
            let name = parts.next()?.trim();
            let value = parts.next()?.trim();
            if name == key && !value.is_empty() {
                Some(value.to_string())
            } else {
                None
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn parses_example_config() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("bids.example.toml");
        let config = load_config(path).expect("should parse example config");

        assert_eq!(config.bid.max_bid, 10.5);
        assert_eq!(config.bid.amount, 3.0);
        assert_eq!(
            config.bid.owner.as_deref(),
            Some("0xabc1230000000000000000000000000000000000")
        );
    }

    #[test]
    fn resolves_owner_from_env_when_not_in_config() {
        let mut config = BidsConfig {
            bid: BidConfig {
                max_bid: 1.0,
                amount: 1.0,
                owner: None,
            },
        };
        // SAFETY: test process controls its own environment and uses a unique key.
        unsafe { env::set_var(PRIVATE_KEY_ENV, "0xfromenv") };
        let bid = resolve_bid(
            &config,
            BidOverrides {
                owner: None,
                ..Default::default()
            },
        )
        .expect("should pick up env owner");
        assert_eq!(bid.owner, "0xfromenv");
        // SAFETY: test process controls its own environment and uses a unique key.
        unsafe { env::remove_var(PRIVATE_KEY_ENV) };

        // Also ensure config owner is used when present.
        config.bid.owner = Some("0xfromconfig".into());
        let bid = resolve_bid(&config, BidOverrides::default()).expect("should use config owner");
        assert_eq!(bid.owner, "0xfromconfig");
    }
}
