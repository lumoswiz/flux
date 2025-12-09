// src/abi/cca.rs
use alloy::sol;

sol! {
    // ---------- Types from libraries ----------

    /// AuctionParameters from IContinuousClearingAuction.sol
    struct AuctionParameters {
        address currency;
        address tokensRecipient;
        address fundsRecipient;
        uint64 startBlock;
        uint64 endBlock;
        uint64 claimBlock;
        uint256 tickSpacing;
        address validationHook;
        uint256 floorPrice;
        uint128 requiredCurrencyRaised;
        bytes auctionStepsData;
    }

    /// Tick from ITickStorage.sol
    struct Tick {
        uint256 next;
        uint256 currencyDemandQ96;
    }

    /// ValueX7 from ValueX7Lib.sol
    /// (aliased uint256 with implicit *1e7 scaling)
    type ValueX7 is uint256;

    /// Bid – from BidLib.sol
    struct Bid {
        uint64 startBlock; // Block number when the bid was first made in
        uint24 startCumulativeMps; // Cumulative mps at the start of the bid
        uint64 exitedBlock; // Block number when the bid was exited
        uint256 maxPrice; // The max price of the bid
        address owner; // Who will receive the tokens filled and currency refunded
        uint256 amountQ96; // User's currency amount in Q96 form
        uint256 tokensFilled; // Amount of tokens filled
    }

    /// Checkpoint – from CheckpointLib.sol (TODO: paste real definition)
    struct Checkpoint {
        uint256 clearingPrice; // The X96 price which the auction is currently clearing at
        ValueX7 currencyRaisedAtClearingPriceQ96_X7; // The currency raised so far to this clearing price
        uint256 cumulativeMpsPerPrice; // A running sum of the ratio between mps and price
        uint24 cumulativeMps; // The number of mps sold in the auction so far (via the original supply schedule)
        uint64 prev; // Block number of the previous checkpoint
        uint64 next; // Block number of the next checkpoint
    }

    /// AuctionStep – from StepLib.sol (TODO if you care about step())
    struct AuctionStep {
        uint24 mps; // Mps to sell per block in the step
        uint64 startBlock; // Start block of the step (inclusive)
        uint64 endBlock; // Ending block of the step (exclusive)
    }

    /// Currency – from CurrencyLibrary.sol (TODO)
    type Currency is address;

    // ---------- External interfaces used by CCA ----------

    /// Minimal ERC20 from external/IERC20Minimal.sol
    interface IERC20Minimal {
        function balanceOf(address account) external view returns (uint256);
        function transfer(address recipient, uint256 amount) external returns (bool);
        function approve(address spender, uint256 amount) external returns (bool);
    }

    /// IValidationHook from IValidationHook.sol
    interface IValidationHook {
        function validate(
            uint256 maxPrice,
            uint128 amount,
            address owner,
            address sender,
            bytes calldata hookData
        ) external;
    }

    /// IDistributionContract from external/IDistributionContract.sol
    interface IDistributionContract {
        function onTokensReceived() external;
    }

    // ---------- Storage interfaces ----------

    /// IBidStorage from IBidStorage.sol
    #[sol(rpc)]
    interface IBidStorage {
        error BidIdDoesNotExist(uint256 bidId);

        function nextBidId() external view returns (uint256);
        function bids(uint256 bidId) external view returns (Bid memory);
    }

    /// ICheckpointStorage from ICheckpointStorage.sol
    interface ICheckpointStorage {
        error CheckpointBlockNotIncreasing();

        function latestCheckpoint() external view returns (Checkpoint memory);
        function clearingPrice() external view returns (uint256);
        function lastCheckpointedBlock() external view returns (uint64);
        function checkpoints(uint64 blockNumber) external view returns (Checkpoint memory);
    }

    /// IStepStorage from IStepStorage.sol
    #[sol(rpc)]
    interface IStepStorage {
        error InvalidEndBlock();
        error AuctionIsOver();
        error InvalidAuctionDataLength();
        error StepBlockDeltaCannotBeZero();
        error InvalidStepDataMps(uint256 actualMps, uint256 expectedMps);
        error InvalidEndBlockGivenStepData(uint64 actualEndBlock, uint64 expectedEndBlock);

        function startBlock() external view returns (uint64);
        function endBlock() external view returns (uint64);

        function pointer() external view returns (address);
        function step() external view returns (AuctionStep memory);

        event AuctionStepRecorded(uint256 startBlock, uint256 endBlock, uint24 mps);
    }

    /// ITickStorage from ITickStorage.sol
    interface ITickStorage {
        error TickSpacingTooSmall();
        error FloorPriceIsZero();
        error FloorPriceTooLow();
        error TickPreviousPriceInvalid();
        error TickPriceNotIncreasing();
        error TickPriceNotAtBoundary();
        error InvalidTickPrice();
        error CannotUpdateUninitializedTick();

        event TickInitialized(uint256 price);
        event NextActiveTickUpdated(uint256 price);

        function nextActiveTickPrice() external view returns (uint256);
        function floorPrice() external view returns (uint256);
        function tickSpacing() external view returns (uint256);
        function ticks(uint256 price) external view returns (Tick memory);
    }

    /// ITokenCurrencyStorage from ITokenCurrencyStorage.sol
    #[sol(rpc)]
    interface ITokenCurrencyStorage {
        error TokenIsAddressZero();
        error TokenAndCurrencyCannotBeTheSame();
        error TotalSupplyIsZero();
        error TotalSupplyIsTooLarge();
        error FundsRecipientIsZero();
        error TokensRecipientIsZero();
        error CannotSweepCurrency();
        error CannotSweepTokens();
        error NotGraduated();

        event TokensSwept(address indexed tokensRecipient, uint256 tokensAmount);
        event CurrencySwept(address indexed fundsRecipient, uint256 currencyAmount);

        function currency() external view returns (Currency);
        function token() external view returns (IERC20Minimal);
        function totalSupply() external view returns (uint128);
        function tokensRecipient() external view returns (address);
        function fundsRecipient() external view returns (address);
    }

    // ---------- Main IContinuousClearingAuction interface ----------
    #[sol(rpc)]
    interface IContinuousClearingAuction is
        IDistributionContract,
        ICheckpointStorage,
        ITickStorage,
        IStepStorage,
        ITokenCurrencyStorage,
        IBidStorage
    {
        // Errors
        error InvalidTokenAmountReceived();
        error InvalidAmount();
        error BidOwnerCannotBeZeroAddress();
        error BidMustBeAboveClearingPrice();
        error InvalidBidPriceTooHigh(uint256 maxPrice, uint256 maxBidPrice);
        error BidAmountTooSmall();
        error CurrencyIsNotNative();
        error AuctionNotStarted();
        error TokensNotReceived();
        error ClaimBlockIsBeforeEndBlock();
        error FloorPriceAndTickSpacingGreaterThanMaxBidPrice(uint256 nextTick, uint256 maxBidPrice);
        error FloorPriceAndTickSpacingTooLarge();
        error BidAlreadyExited();
        error CannotExitBid();
        error CannotPartiallyExitBidBeforeEndBlock();
        error InvalidLastFullyFilledCheckpointHint();
        error InvalidOutbidBlockCheckpointHint();
        error NotClaimable();
        error BatchClaimDifferentOwner(address expectedOwner, address receivedOwner);
        error BidNotExited();
        error CannotPartiallyExitBidBeforeGraduation();
        error TokenTransferFailed();
        error AuctionIsNotOver();
        error InvalidBidUnableToClear();
        error AuctionSoldOut();

        // Events
        event TokensReceived(uint256 totalSupply);
        event BidSubmitted(uint256 indexed id, address indexed owner, uint256 price, uint128 amount);
        event CheckpointUpdated(uint256 blockNumber, uint256 clearingPrice, uint24 cumulativeMps);
        event ClearingPriceUpdated(uint256 blockNumber, uint256 clearingPrice);
        event BidExited(uint256 indexed bidId, address indexed owner, uint256 tokensFilled, uint256 currencyRefunded);
        event TokensClaimed(uint256 indexed bidId, address indexed owner, uint256 tokensFilled);

        // Entrypoints
        function submitBid(
            uint256 maxPrice,
            uint128 amount,
            address owner,
            uint256 prevTickPrice,
            bytes calldata hookData
        ) external payable returns (uint256 bidId);

        function submitBid(
            uint256 maxPrice,
            uint128 amount,
            address owner,
            bytes calldata hookData
        ) external payable returns (uint256 bidId);

        function checkpoint() external returns (Checkpoint memory _checkpoint);

        function isGraduated() external view returns (bool);
        function currencyRaised() external view returns (uint256);

        function exitBid(uint256 bidId) external;
        function exitPartiallyFilledBid(
            uint256 bidId,
            uint64 lastFullyFilledCheckpointBlock,
            uint64 outbidBlock
        ) external;

        function claimTokens(uint256 bidId) external;
        function claimTokensBatch(address owner, uint256[] calldata bidIds) external;

        function sweepCurrency() external;
        function claimBlock() external view returns (uint64);
        function validationHook() external view returns (IValidationHook);
        function sweepUnsoldTokens() external;

        function currencyRaisedQ96_X7() external view returns (ValueX7);
        function sumCurrencyDemandAboveClearingQ96() external view returns (uint256);
        function totalClearedQ96_X7() external view returns (ValueX7);
        function totalCleared() external view returns (uint256);
    }
}
