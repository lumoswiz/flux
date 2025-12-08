// src/domain/price.rs

use alloy::primitives::U256;
use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use thiserror::Error;

/// 2^96, used for Uniswap-style Q96 fixed point prices.
pub const Q96: U256 = U256::from_limbs([0, 0, 1 << (96 - 64), 0]);

/// Strongly-typed Q96 price (currency per token).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PriceQ96(pub U256);

#[derive(Debug, Error)]
pub enum PriceError {
    #[error("invalid price {0}")]
    InvalidPrice(f64),
    #[error("overflow while converting price to Q96")]
    Overflow,
}

pub type PriceResult<T> = Result<T, PriceError>;

/// Convert a human price (currency_per_token) to Q96.
///
/// price_human:
///   - expressed as currency per token (e.g. 0.5 USDC per TOKEN)
/// token_decimals / currency_decimals:
///   - ERC-20 decimals of token and currency.
pub fn q96_from_ratio(
    price_human: f64,
    token_decimals: u8,
    currency_decimals: u8,
) -> PriceResult<U256> {
    let p = Decimal::from_f64(price_human).ok_or(PriceError::InvalidPrice(price_human))?;

    let token_scale = Decimal::from_i128_with_scale(10_i128.pow(token_decimals as u32) as i128, 0);
    let currency_scale =
        Decimal::from_i128_with_scale(10_i128.pow(currency_decimals as u32) as i128, 0);

    let scale = token_scale / currency_scale;
    let q96_factor = Decimal::from_u128(1u128 << 96).unwrap();

    let v = (p * scale * q96_factor)
        .to_u128()
        .ok_or(PriceError::Overflow)?;

    Ok(U256::from(v))
}

/// Convert a Q96 price back to a human float (for display only).
pub fn ratio_from_q96(price_q96: U256, token_decimals: u8, currency_decimals: u8) -> f64 {
    let raw = price_q96.to::<u128>() as f64;
    let q96 = (1u128 << 96) as f64;
    let scale = 10f64.powi((token_decimals as i32) - (currency_decimals as i32));
    (raw / q96) / scale
}

impl PriceQ96 {
    pub fn from_ratio(
        price_human: f64,
        token_decimals: u8,
        currency_decimals: u8,
    ) -> PriceResult<Self> {
        Ok(Self(q96_from_ratio(
            price_human,
            token_decimals,
            currency_decimals,
        )?))
    }

    pub fn to_ratio(&self, token_decimals: u8, currency_decimals: u8) -> f64 {
        ratio_from_q96(self.0, token_decimals, currency_decimals)
    }
}
