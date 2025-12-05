use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use flux_cli::{BidOverrides, DEFAULT_CONFIG_PATH, load_config, resolve_bid};

#[derive(Debug, Parser)]
#[command(name = "flux-cli", about = "Flux bidding CLI", version)]
struct Cli {
    /// Path to the bids configuration file
    #[arg(short, long, default_value = DEFAULT_CONFIG_PATH, value_name = "FILE")]
    config: PathBuf,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Submit or preview a bid
    Bids(BidArgs),
}

#[derive(Debug, Args)]
struct BidArgs {
    /// Maximum bid price
    #[arg(long, value_name = "AMOUNT")]
    max_bid: Option<f64>,
    /// Bid amount
    #[arg(long, value_name = "AMOUNT")]
    amount: Option<f64>,
    /// Bid owner/private key
    #[arg(long, value_name = "KEY")]
    owner: Option<String>,
}

fn main() {
    let cli = Cli::parse();

    let config = match load_config(&cli.config) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    };

    match cli.command {
        Some(Commands::Bids(args)) => handle_bids(&config, args),
        None => {
            println!("Loaded config from {}", cli.config.display());
        }
    }
}

fn handle_bids(config: &flux_cli::BidsConfig, args: BidArgs) {
    let overrides = BidOverrides {
        max_bid: args.max_bid,
        amount: args.amount,
        owner: args.owner,
    };

    match resolve_bid(config, overrides) {
        Ok(bid) => println!(
            "Bid ready: max_bid={}, amount={}, owner={}",
            bid.max_bid, bid.amount, bid.owner
        ),
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
}
