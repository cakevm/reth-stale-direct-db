//! Block monitoring loop that compares RPC blocks with database state.

use alloy_eips::BlockNumHash;
use alloy_primitives::B256;
use alloy_provider::{Provider, ProviderBuilder, WsConnect};
use eyre::Result;
use futures::StreamExt;
use futures::stream::select;
use reth_ethereum::primitives::AlloyBlockHeader;
use reth_ethereum::provider::{BlockHashReader, BlockNumReader, HeaderProvider};
use reth_provider::ProviderFactory;
use reth_provider::providers::ProviderNodeTypes;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

use crate::consistency::check_block_hash_reader_health;
use crate::sync::wait_for_sync;

/// Event from either subscription stream
enum BlockEvent {
    /// New block header from eth_subscribe newHeads
    Header { number: u64, hash: B256 },
    /// Block persisted to DB from reth_subscribeLatestPersistedBlock
    Persisted { number: u64, hash: B256 },
}

/// Run the block monitoring loop.
///
/// Subscribes to new blocks via WebSocket RPC and for each block:
/// 1. Runs consistency check (verifies last 256 block hashes are accessible)
/// 2. Compares RPC block hash with database block hash
/// 3. Exits if consistency check fails
///
/// If `subscribe_persisted_blocks` is true, also subscribes to reth_subscribeLatestPersistedBlock
/// and waits for persistence events before processing accumulated headers.
pub async fn run_monitor<N>(
    rpc_ws: &str,
    factory: ProviderFactory<N>,
    subscribe_persisted_blocks: bool,
) -> Result<()>
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

    // Always subscribe to block headers
    info!("Subscribing to newHeads...");
    let header_sub = rpc_provider.subscribe_blocks().await?;
    let header_stream = header_sub.into_stream().map(|h| BlockEvent::Header {
        number: h.inner.number,
        hash: h.hash,
    });

    if subscribe_persisted_blocks {
        // Also subscribe to persisted blocks
        info!("Subscribing to reth_subscribeLatestPersistedBlock...");
        let persisted_sub = rpc_provider
            .subscribe_to::<BlockNumHash>("reth_subscribeLatestPersistedBlock")
            .await?;
        let persisted_stream = persisted_sub.into_stream().map(|b| BlockEvent::Persisted {
            number: b.number,
            hash: b.hash,
        });

        // Merge both streams
        let mut combined = select(header_stream, persisted_stream);
        info!("Subscribed to both streams, waiting...");

        // Buffer for pending headers (block_number -> (hash, arrival_time))
        let mut pending_headers: BTreeMap<u64, (B256, Instant)> = BTreeMap::new();

        while let Some(event) = combined.next().await {
            match event {
                BlockEvent::Header { number, hash } => {
                    info!(
                        block_number = number,
                        %hash,
                        pending = pending_headers.len(),
                        "New head received"
                    );
                    pending_headers.insert(number, (hash, Instant::now()));
                }
                BlockEvent::Persisted { number, hash } => {
                    // Collect blocks to process (all <= persisted block number)
                    let blocks_to_process: Vec<_> = pending_headers
                        .range(..=number)
                        .map(|(&n, &(h, t))| (n, h, t))
                        .collect();

                    // Calculate max latency from header arrival to persistence
                    let max_latency = blocks_to_process
                        .iter()
                        .map(|(_, _, t)| t.elapsed())
                        .max()
                        .unwrap_or(Duration::ZERO);

                    info!(
                        block_number = number,
                        %hash,
                        blocks_to_flush = blocks_to_process.len(),
                        max_latency_ms = max_latency.as_millis(),
                        "New latest persisted block received"
                    );

                    // Process all buffered headers up to persisted block
                    for (block_number, block_hash, arrival_time) in blocks_to_process {
                        let latency = arrival_time.elapsed();
                        process_block(block_number, block_hash, latency, &factory)?;
                        pending_headers.remove(&block_number);
                    }
                }
            }
        }
    } else {
        // Simple mode: process headers immediately
        let mut stream = header_stream;
        info!("Subscribed to new blocks, waiting...");

        while let Some(BlockEvent::Header { number, hash }) = stream.next().await {
            process_block(number, hash, Duration::ZERO, &factory)?;
        }
    }

    Ok(())
}

/// Process a block event from either subscription type
fn process_block<N>(
    block_number: u64,
    rpc_block_hash: B256,
    latency: Duration,
    factory: &ProviderFactory<N>,
) -> Result<()>
where
    N: ProviderNodeTypes,
{
    info!(
        block_number,
        %rpc_block_hash,
        latency_ms = latency.as_millis(),
        "Verifying block"
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
            "Reth must be running with `--engine.persistence-threshold 0` to ensure immediate DB writes"
        );
        return Ok(());
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

    Ok(())
}
