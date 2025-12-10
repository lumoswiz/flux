use std::ops::{Add, AddAssign};

use alloy::primitives::{Address, U256, aliases::U24};

#[derive(Clone, Copy, Debug)]
pub struct TickSpacing(U256);

impl TickSpacing {
    pub const MIN: u32 = 2;

    pub fn new(value: U256) -> Self {
        Self(value)
    }

    pub fn min() -> Self {
        Self(U256::from(Self::MIN))
    }

    pub fn as_u256(&self) -> U256 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Price(U256);

impl Price {
    pub const ZERO: Self = Self(U256::ZERO);

    pub fn new(value: U256) -> Self {
        Self(value)
    }

    pub fn as_u256(&self) -> U256 {
        self.0
    }

    pub fn is_aligned(&self, tick_spacing: TickSpacing) -> bool {
        self.0 % tick_spacing.0 == U256::ZERO
    }

    pub fn clamp_to_nearest_tick(
        &self,
        tick_spacing: TickSpacing,
        floor: Price,
        cap: Price,
    ) -> Self {
        if self.0 >= cap.0 {
            return cap;
        }
        if self.0 <= floor.0 {
            return floor;
        }
        let spacing = tick_spacing.0;
        let offset = self.0 - floor.0;
        let rem = offset % spacing;
        if rem.is_zero() {
            return *self;
        }
        let down = self.0 - rem;
        let up = down + spacing;
        let choose_up = rem > spacing - rem;
        let candidate = if choose_up { up } else { down };
        Self(candidate.min(cap.0))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CurrencyAmount(U256);

impl CurrencyAmount {
    pub const ZERO: Self = Self(U256::ZERO);

    pub fn new(value: U256) -> Self {
        Self(value)
    }

    pub fn as_u128(&self) -> u128 {
        self.0.to::<u128>()
    }

    pub fn as_u256(&self) -> U256 {
        self.0
    }

    pub fn is_zero(&self) -> bool {
        self.0 == U256::ZERO
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct TokenAmount(U256);

impl TokenAmount {
    pub const ZERO: Self = Self(U256::ZERO);

    pub fn new(value: U256) -> Self {
        Self(value)
    }

    pub fn as_u256(&self) -> U256 {
        self.0
    }

    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }
}

impl Add for TokenAmount {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign for TokenAmount {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BidId(U256);

impl BidId {
    pub fn new(value: U256) -> Self {
        Self(value)
    }

    pub fn as_u256(&self) -> U256 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BlockNumber(u64);

impl BlockNumber {
    pub const TAIL_SENTINEL: Self = Self(u64::MAX);

    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Mps(U24);

impl Mps {
    pub const FULL: u32 = 10_000_000;
    pub const ZERO: u32 = 0;

    pub fn new(value: U24) -> Self {
        Self(value)
    }

    pub fn as_u24(&self) -> U24 {
        self.0
    }

    pub fn remaining(&self) -> Self {
        Self(U24::from(Self::FULL) - self.0)
    }

    pub fn is_sold_out(&self) -> bool {
        self.0 >= U24::from(Self::FULL)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CurrencyAddr(Address);

impl CurrencyAddr {
    pub fn new(value: Address) -> Self {
        Self(value)
    }

    pub fn as_address(&self) -> Address {
        self.0
    }

    pub fn is_native(&self) -> bool {
        self.0 == Address::ZERO
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TokenAddr(Address);

impl TokenAddr {
    pub fn new(value: Address) -> Self {
        Self(value)
    }

    pub fn as_address(&self) -> Address {
        self.0
    }
}

#[derive(Clone, Copy, Debug)]
pub struct HookAddr(Address);

impl HookAddr {
    pub fn new(value: Address) -> Self {
        Self(value)
    }

    pub fn as_address(&self) -> Address {
        self.0
    }

    pub fn is_configured(&self) -> bool {
        self.0 != Address::ZERO
    }
}
