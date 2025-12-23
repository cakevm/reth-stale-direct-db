//! Database consistency check based on rbuilder's implementation.
//!
//! See: https://github.com/flashbots/rbuilder/blob/95323c/crates/rbuilder/src/utils/provider_factory_reopen.rs#L157
//! See: https://github.com/paradigmxyz/reth/issues/7836

use reth_ethereum::provider::BlockHashReader;

#[derive(Debug, thiserror::Error)]
pub enum HistoricalBlockError {
    #[error("Provider error: {0}")]
    ProviderError(#[from] reth_provider::ProviderError),
    #[error(
        "Missing historical block hash for block {missing_hash_block}, latest block: {latest_block}"
    )]
    MissingHash {
        missing_hash_block: u64,
        latest_block: u64,
    },
}

/// Check if we have all necessary historical block hashes (last 256 blocks).
///
/// EVM needs access to block hashes of the previous 256 blocks for the BLOCKHASH opcode.
/// If any of these hashes are missing, the database is in an inconsistent state.
pub fn check_block_hash_reader_health<R: BlockHashReader>(
    last_block_number: u64,
    reader: &R,
) -> Result<(), HistoricalBlockError> {
    let blocks_to_check = last_block_number.min(256);
    for i in 0..blocks_to_check {
        let num = last_block_number - i;
        let hash = reader.block_hash(num)?;
        if hash.is_none() {
            return Err(HistoricalBlockError::MissingHash {
                missing_hash_block: num,
                latest_block: last_block_number,
            });
        }
    }
    Ok(())
}
