use alloy::primitives::U256;

pub struct TickSpacing(U256);

impl TickSpacing {
    pub const ZERO: Self = Self(U256::ZERO);
    pub const MIN: u32 = 2;

    pub fn new(value: U256) -> Self {
        Self(value)
    }

    pub fn as_u256(&self) -> U256 {
        self.0
    }

    pub fn min() -> Self {
        Self(U256::from(Self::MIN))
    }
}

#[derive(Clone, Copy)]
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

pub struct CurrencyAmount(u128);

impl CurrencyAmount {
    pub const ZERO: Self = Self(0);

    pub fn new(value: u128) -> Self {
        Self(value)
    }

    pub fn as_u128(&self) -> u128 {
        self.0
    }

    pub fn is_zero(&self) -> bool {
        self.0 == 0
    }
}

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

pub struct BidId(U256);

impl BidId {
    pub fn new(value: U256) -> Self {
        Self(value)
    }

    pub fn as_u256(&self) -> U256 {
        self.0
    }
}

pub struct BlockNumber(u64);

impl BlockNumber {
    pub const MAX: Self = Self(u64::MAX);

    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

pub struct Mps(u32);

impl Mps {
    pub const FULL: Self = Self(10_000_000);
    pub const ZERO: Self = Self(0);

    pub fn new(value: u32) -> Self {
        Self(value)
    }

    pub fn as_u32(&self) -> u32 {
        self.0
    }

    pub fn remaining(&self) -> Self {
        Self(Self::FULL.0 - self.0)
    }

    pub fn is_sold_out(&self) -> bool {
        self.0 >= Self::FULL.0
    }
}
