use alloy::primitives::Address;

/// Information about the currency used in a CCA auction.
///
/// In the contracts:
/// - currency() returns a "Currency" type which is just an address.
/// - address(0) => native (ETH / chain native)
/// - non-zero   => ERC-20 at that address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurrencyInfo {
    pub address: Address,
}

impl CurrencyInfo {
    pub fn new(address: Address) -> Self {
        Self { address }
    }

    /// Returns true if this auction uses the chain's native asset.
    pub fn is_native(&self) -> bool {
        self.address.is_zero()
    }
}
