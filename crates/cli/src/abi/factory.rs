use alloy::sol;

sol! {
    interface IDistributionContract {
        function onTokensReceived() external;
    }

    interface IDistributionStrategy {
        function initializeDistribution(
            address token,
            uint256 amount,
            bytes calldata configData,
            bytes32 salt
        ) external returns (IDistributionContract distributionContract);
    }

    interface IContinuousClearingAuctionFactory is IDistributionStrategy {
        error InvalidTokenAmount(uint256 amount);

        event AuctionCreated(
            address indexed auction,
            address indexed token,
            uint256 amount,
            bytes configData
        );

        function getAuctionAddress(
            address token,
            uint256 amount,
            bytes calldata configData,
            bytes32 salt,
            address sender
        ) external view returns (address);
    }
}
