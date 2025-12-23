//! Sync status check - waits for node to finish syncing.

use alloy_provider::Provider;
use alloy_rpc_types_eth::SyncStatus;
use eyre::Result;
use std::time::Duration;
use tokio::time::sleep;
use tracing::info;

/// Wait for the node to finish syncing.
///
/// Polls every `poll_interval` until the node reports it's no longer syncing.
pub async fn wait_for_sync<P: Provider>(provider: &P, poll_interval: Duration) -> Result<()> {
    loop {
        let sync_status = provider.syncing().await?;

        match sync_status {
            SyncStatus::None => {
                info!("Node is synced");
                return Ok(());
            }
            SyncStatus::Info(info) => {
                info!(
                    current_block = %info.current_block,
                    highest_block = %info.highest_block,
                    "Node is syncing, waiting..."
                );
            }
        }

        sleep(poll_interval).await;
    }
}
