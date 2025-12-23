mod cli;
mod consistency;
mod monitor;
mod sync;

use clap::Parser;
use eyre::Result;
use reth_ethereum::{node::EthereumNode, provider::providers::ReadOnlyConfig};
use tracing::info;

use crate::cli::{Args, get_chain_spec};
use crate::monitor::run_monitor;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("reth_stale_direct_db=info".parse()?),
        )
        .init();

    let args = Args::parse();

    info!("Starting reth-stale-direct-db");
    info!("RPC WebSocket: {}", args.rpc_ws);
    info!("DB Path: {:?}", args.db_path);
    info!("Chain: {:?}", args.chain);

    // Get chain spec
    let chain_spec = get_chain_spec(args.chain);

    // Open database in read-only mode
    let factory = EthereumNode::provider_factory_builder()
        .open_read_only(chain_spec, ReadOnlyConfig::from_datadir(&args.db_path))?;
    info!("Database opened successfully");

    // Run the monitoring loop
    run_monitor(&args.rpc_ws, factory).await
}
