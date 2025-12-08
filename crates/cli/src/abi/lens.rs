use alloy::sol;

sol! {
    // Re-declare Checkpoint struct as used by the lens.
    //
    // Note: The Solidity version uses:
    //   struct Checkpoint {
    //       uint256 clearingPrice;
    //       ValueX7 currencyRaisedAtClearingPriceQ96_X7;
    //       uint256 cumulativeMpsPerPrice;
    //       uint24 cumulativeMps;
    //       uint64 prev;
    //       uint64 next;
    //   }
    //
    // We can safely treat ValueX7 as a uint256 here for ABI purposes.
    struct Checkpoint {
        uint256 clearingPrice;
        uint256 currencyRaisedAtClearingPriceQ96_X7;
        uint256 cumulativeMpsPerPrice;
        uint24 cumulativeMps;
        uint64 prev;
        uint64 next;
    }

    struct AuctionState {
        Checkpoint checkpoint;
        uint256 currencyRaised;
        uint256 totalCleared;
        bool isGraduated;
    }

    // ABI-wise, an interface param is just an address.
    // Also note: no "memory" in the return type here; sol! wants `AuctionState`.
    #[sol(rpc)]
    interface IAuctionStateLens {
        function state(address auction) external returns (AuctionState);
    }
}
