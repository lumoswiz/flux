# Flux CLI

A minimal CLI for working with bids.

## Usage

Run from the workspace root:

- Show help/version: `cargo run -p flux-cli -- --help` / `--version`
- Default config (`bids.toml`): `cargo run -p flux-cli --`
- Bids subcommand with overrides: `cargo run -p flux-cli -- bids --max_bid 5.5 --amount 2 --owner 0xabc`
- Use the example config: `cargo run -p flux-cli -- --config crates/cli/bids.example.toml bids`
