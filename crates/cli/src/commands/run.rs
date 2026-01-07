use std::sync::Arc;

use alloy::{
    network::EthereumWallet,
    primitives::Address,
    providers::ProviderBuilder,
    signers::local::LocalSigner,
};
use eyre::{Result, eyre};
use tracing::info;

use crate::{
    config::{get_rpc_url, load_bids_config},
    runner::Runner,
    state::BidTracker,
};

/// NoOp validation hook for auctions without custom hooks
struct NoOpHook;

impl flux_core::ValidationHook for NoOpHook {}

/// Execute the run command
pub async fn execute(auction: String, keystore: String, config_path: String) -> Result<()> {
    // Load configuration
    let rpc_url = get_rpc_url()?;
    let bids_config = load_bids_config(&config_path)?;

    info!("Loaded {} bid(s) from config", bids_config.bids.len());

    // Parse auction address
    let auction_address: Address = auction
        .parse()
        .map_err(|e| eyre!("Invalid auction address '{}': {}", auction, e))?;

    // Load keystore
    let password = rpassword::prompt_password("Enter keystore password: ")?;
    let signer = LocalSigner::decrypt_keystore(&keystore, &password)
        .map_err(|e| eyre!("Failed to decrypt keystore: {}", e))?;

    let owner_address = signer.address();
    info!("Loaded wallet: {}", owner_address);

    // Create provider with signer
    let wallet = EthereumWallet::from(signer);
    let provider = ProviderBuilder::new()
        .wallet(wallet)
        .connect(&rpc_url)
        .await
        .map_err(|e| eyre!("Failed to connect to RPC: {}", e))?;

    // Create auction client
    let client = flux_core::AuctionClient::new(
        provider.clone(),
        auction_address,
        owner_address,
        Arc::new(NoOpHook) as Arc<dyn flux_core::ValidationHook>,
        vec![], // No previously tracked bids
    )
    .await
    .map_err(|e| eyre!("Failed to create auction client: {}", e))?;

    // Check for validation hook and warn
    let config = client.config();
    if config.validation_hook.is_configured() {
        tracing::warn!(
            "Auction has validation hook at {} - bids may fail without proper hook data",
            config.validation_hook.as_address()
        );
    }

    info!("Connected to auction at {}", auction_address);
    info!(
        "Auction blocks: start={}, end={}, claim={}",
        config.start_block.as_u64(),
        config.end_block.as_u64(),
        config.claim_block.as_u64()
    );

    // Initialize bid tracker with configured bids
    let mut tracker = BidTracker::new();
    for bid_config in &bids_config.bids {
        let max_price = bid_config.max_price()?;
        let amount = bid_config.amount()?;
        tracker.add_pending(max_price, amount);
        info!(
            "Queued bid: max_price={}, amount={}",
            bid_config.max_price, bid_config.amount
        );
    }

    // Create executor
    let executor = flux_core::IntentExecutor::new(client);

    // Create and run the runner
    let mut runner = Runner::new(executor, tracker, provider);
    runner.run().await?;

    Ok(())
}
