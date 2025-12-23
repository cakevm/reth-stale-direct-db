//! Block monitoring loop that compares RPC blocks with database state.

use alloy_provider::{Provider, ProviderBuilder, WsConnect};
use eyre::Result;
use futures::StreamExt;
use reth_ethereum::primitives::AlloyBlockHeader;
use reth_ethereum::provider::{BlockHashReader, BlockNumReader, HeaderProvider};
use reth_provider::ProviderFactory;
use reth_provider::providers::ProviderNodeTypes;
use std::time::Duration;
use tracing::{debug, info, warn};

use crate::consistency::check_block_hash_reader_health;
use crate::sync::wait_for_sync;

/// Run the block monitoring loop.
///
/// Subscribes to new blocks via WebSocket RPC and for each block:
/// 1. Runs consistency check (verifies last 256 block hashes are accessible)
/// 2. Compares RPC block hash with database block hash
/// 3. Exits if consistency check fails
pub async fn run_monitor<N>(rpc_ws: &str, factory: ProviderFactory<N>) -> Result<()>
where
    N: ProviderNodeTypes,
{
    // Connect to WebSocket RPC
    info!("Connecting to WebSocket RPC...");
    let ws = WsConnect::new(rpc_ws);
    let rpc_provider = ProviderBuilder::new().connect_ws(ws).await?;
    info!("Connected to WebSocket RPC");

    // Wait for node to sync
    wait_for_sync(&rpc_provider, Duration::from_secs(5)).await?;

    // Subscribe to new blocks
    let sub = rpc_provider.subscribe_blocks().await?;
    let mut stream = sub.into_stream();
    info!("Subscribed to new blocks, waiting...");

    // Main loop
    while let Some(rpc_header) = stream.next().await {
        let block_number = rpc_header.inner.number;
        let rpc_block_hash = rpc_header.hash;

        info!(
            block_number,
            %rpc_block_hash,
            "New block from RPC"
        );

        // Get provider for reads (opens RO transaction)
        let provider = factory.provider()?;

        // Read last block number from database
        let db_last_block = provider.last_block_number()?;

        // The database might be slightly behind the RPC
        if db_last_block < block_number {
            // Still read the latest block hash from DB to verify DB access works
            let db_block_hash = provider.block_hash(db_last_block)?.ok_or_else(|| {
                eyre::eyre!("Block hash not found for db_last_block {}", db_last_block)
            })?;
            info!(
                db_last_block,
                %db_block_hash,
                rpc_block_number = block_number,
                "Database is behind RPC, skipping"
            );
            warn!(
                "Reth must be running with `--persistence.threshold 0` to ensure immediate DB writes"
            );
            continue;
        }

        // Run consistency check - exit if it fails
        check_block_hash_reader_health(db_last_block, &provider)
            .map_err(|e| eyre::eyre!("Database consistency check failed: {}", e))?;

        info!(
            block_number,
            db_last_block, "Consistency check passed (256 block hashes accessible)"
        );

        // Read block hash from database
        let db_block_hash = provider
            .block_hash(block_number)?
            .ok_or_else(|| eyre::eyre!("Block hash not found for block {}", block_number))?;

        if db_block_hash == rpc_block_hash {
            info!(
                block_number,
                %db_block_hash,
                "Block hash matches"
            );
        } else {
            return Err(eyre::eyre!(
                "Block hash MISMATCH at block {}: RPC={} DB={}",
                block_number,
                rpc_block_hash,
                db_block_hash
            ));
        }

        // Also read header for additional verification
        if let Some(header) = provider.header_by_number(block_number)? {
            debug!(
                block_number,
                gas_used = header.gas_used(),
                gas_limit = header.gas_limit(),
                timestamp = header.timestamp(),
                "Block header from DB"
            );
        }
    }

    Ok(())
}
