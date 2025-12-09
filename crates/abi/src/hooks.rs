use alloy::sol;

sol! {
    interface IValidationHook {
        function validate(
            uint256 maxPrice,
            uint128 amount,
            address owner,
            address sender,
            bytes calldata hookData
        ) external;
    }
}
