mod commands;
mod config;
mod runner;
mod state;

use clap::{Parser, Subcommand};
use eyre::Result;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[derive(Parser)]
#[command(name = "flux")]
#[command(about = "CLI for participating in Uniswap Continuous Clearing Auctions")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the automated auction runner
    Run {
        /// Auction contract address
        #[arg(long)]
        auction: String,

        /// Path to keystore JSON file
        #[arg(long)]
        keystore: String,

        /// Path to bids config file
        #[arg(long, default_value = "./bids.toml")]
        config: String,
    },

    /// Account management
    Account {
        #[command(subcommand)]
        command: AccountCommands,
    },
}

#[derive(Subcommand)]
enum AccountCommands {
    /// Create a new keystore from a private key
    Add {
        /// Output path for keystore file
        #[arg(long)]
        output: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env().add_directive("flux_cli=info".parse()?))
        .init();

    // Load .env file if present
    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            auction,
            keystore,
            config,
        } => {
            commands::run::execute(auction, keystore, config).await?;
        }
        Commands::Account { command } => match command {
            AccountCommands::Add { output } => {
                commands::account::add(output)?;
            }
        },
    }

    Ok(())
}
