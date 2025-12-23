use clap::{Parser, ValueEnum};
use reth_ethereum::chainspec::{ChainSpec, HOLESKY, MAINNET, SEPOLIA};
use std::{path::PathBuf, sync::Arc};

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Chain {
    Mainnet,
    Sepolia,
    Holesky,
}

#[derive(Parser, Debug)]
#[command(name = "reth-stale-direct-db")]
#[command(about = "Minimal Reth direct DB reader with consistency checks")]
pub struct Args {
    /// WebSocket RPC URL for block subscriptions
    #[arg(long)]
    pub rpc_ws: String,

    /// Path to Reth data directory
    #[arg(long)]
    pub db_path: PathBuf,

    /// Chain to use
    #[arg(long, value_enum, default_value = "mainnet")]
    pub chain: Chain,
}

pub fn get_chain_spec(chain: Chain) -> Arc<ChainSpec> {
    match chain {
        Chain::Mainnet => MAINNET.clone(),
        Chain::Sepolia => SEPOLIA.clone(),
        Chain::Holesky => HOLESKY.clone(),
    }
}
