use std::path::PathBuf;

use alloy::primitives::{Address, U256};
use clap::{Args, Parser, Subcommand};

use flux_cli::{
    commands::status as status_cmd,
    config::{BidOverrides, BidsConfig, DEFAULT_CONFIG_PATH, load_config, resolve_bid},
};

#[derive(Debug, Parser)]
#[command(name = "flux-cli", about = "CCA bidding CLI", version)]
struct Cli {
    /// Path to the bids configuration file
    #[arg(short, long, default_value = DEFAULT_CONFIG_PATH, value_name = "FILE")]
    config: PathBuf,

    /// RPC URL for the target chain (only required for on-chain commands like `status`)
    #[arg(long, env = "CCA_RPC_URL", value_name = "URL")]
    rpc_url: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Submit or preview a bid from config (local only for now)
    Bids(BidArgs),

    /// Show on-chain status of a bid in an auction
    Status(StatusArgs),
}

#[derive(Debug, Args)]
struct BidArgs {
    /// Maximum bid price (human units, from config by default)
    #[arg(long, value_name = "AMOUNT")]
    max_bid: Option<f64>,
    /// Bid amount (human units, from config by default)
    #[arg(long, value_name = "AMOUNT")]
    amount: Option<f64>,
    /// Bid owner/private key
    #[arg(long, value_name = "KEY")]
    owner: Option<String>,
}

#[derive(Debug, Args)]
struct StatusArgs {
    /// Address of the AuctionStateLens contract
    #[arg(long, value_name = "ADDRESS")]
    lens: String,

    /// Address of the ContinuousClearingAuction contract
    #[arg(long, value_name = "ADDRESS")]
    auction: String,

    /// Bid id (uint256, decimal or 0x-prefixed hex)
    #[arg(long, value_name = "ID")]
    bid_id: String,
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let cli = Cli::parse();

    // Load config once; still useful for the Bids subcommand
    let config = match load_config(&cli.config) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    };

    match cli.command {
        Some(Commands::Bids(args)) => handle_bids(&config, args),
        Some(Commands::Status(args)) => {
            let rpc_url = cli
                .rpc_url
                .as_deref()
                .ok_or_else(|| eyre::eyre!("--rpc-url or CCA_RPC_URL is required for `status`"))?;

            handle_status(rpc_url, args).await?
        }
        None => {
            println!("Loaded config from {}", cli.config.display());
        }
    }

    Ok(())
}

fn handle_bids(config: &BidsConfig, args: BidArgs) {
    let overrides = BidOverrides {
        max_bid: args.max_bid,
        amount: args.amount,
        owner: args.owner,
    };

    match resolve_bid(config, overrides) {
        Ok(bid) => println!(
            "Bid ready (local): max_bid={}, amount={}, owner={}",
            bid.max_bid, bid.amount, bid.owner
        ),
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
}

async fn handle_status(rpc_url: &str, args: StatusArgs) -> eyre::Result<()> {
    // Parse addresses and bid id
    let lens_addr: Address = args.lens.parse()?;
    let auction_addr: Address = args.auction.parse()?;
    let bid_id_u256: U256 = parse_u256(&args.bid_id)?;

    let output = status_cmd::status(rpc_url, auction_addr, lens_addr, bid_id_u256).await?;
    println!("{output:?}");
    Ok(())
}

fn parse_u256(s: &str) -> eyre::Result<U256> {
    if let Some(stripped) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        Ok(U256::from_str_radix(stripped, 16)?)
    } else {
        Ok(U256::from_str_radix(s, 10)?)
    }
}
