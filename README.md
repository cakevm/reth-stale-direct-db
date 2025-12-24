# reth-stale-direct-db

Minimal example for reading from reth's direct database while subscribing to new block events via WebSocket RPC. Includes database consistency checks based on [rbuilder's `check_block_hash_reader_health`](https://github.com/flashbots/rbuilder/blob/95323c/crates/rbuilder/src/utils/provider_factory_reopen.rs#L157) implementation.

## Problem

When reading from reth's database while the node is running, there can be consistency issues where historical block hashes become temporarily unavailable. This is documented in [reth#7836](https://github.com/paradigmxyz/reth/issues/7836).

### Potential Fix

There's an experimental commit by joshieDo that removes static file caching entirely ("create mmap every time"):
https://github.com/paradigmxyz/reth/commit/fc64ee444e7eb163c9a860bed24c5ebc6966c125

This commit from Jan 31, 2025 is not merged into main - it may be in a feature branch, stashed, or abandoned.

## How It Works

This tool monitors the database consistency by:
1. Subscribing to new blocks via WebSocket RPC
2. Verifying that the last 256 block hashes are accessible in the database (required for EVM's BLOCKHASH opcode)
3. Comparing RPC block hashes with database block hashes
4. **Exiting immediately if consistency check fails**

### Persisted Blocks Mode (`--subscribe-persisted-blocks`)

When enabled, the tool subscribes to both:
- Standard `eth_subscribe newHeads` for new block headers
- Custom `reth_subscribeLatestPersistedBlock` for persistence events

Headers are buffered until a persistence event arrives, then all buffered blocks up to the persisted block number are processed. This mode tracks the latency from header arrival to persistence event, which is useful for measuring how quickly blocks are written to disk.

## Usage

```bash
# Mainnet (default)
cargo run -- --rpc-ws wss://eth-mainnet.example.com --db-path ~/.local/share/reth/mainnet

# Sepolia testnet
cargo run -- --rpc-ws wss://sepolia.example.com --db-path ~/.local/share/reth/sepolia --chain sepolia

# Holesky testnet
cargo run -- --rpc-ws wss://holesky.example.com --db-path ~/.local/share/reth/holesky --chain holesky

# With debug logging
RUST_LOG=debug cargo run -- --rpc-ws wss://eth-mainnet.example.com --db-path ~/.local/share/reth/mainnet

# Use reth_subscribeLatestPersistedBlock (requires https://github.com/cakevm/reth branch subscribe-persisted-block)
# Note: Reth can be running with --engine.persistence-threshold 0 for immediate DB writes
cargo run -- --rpc-ws wss://eth-mainnet.example.com --db-path ~/.local/share/reth/mainnet --subscribe-persisted-blocks
```

## CLI Options

| Option | Description                                   | Default |
|--------|-----------------------------------------------|---------|
| `--rpc-ws` | WebSocket RPC URL for block subscriptions     | Required |
| `--db-path` | Path to reth data directory                   | Required |
| `--chain` | Chain to use: `mainnet`, `sepolia`, `holesky` | `mainnet` |
| `--subscribe-persisted-blocks` | Also subscribe to `reth_subscribeLatestPersistedBlock` and buffer headers until persistence (see above) | `false` |

## Output

On each new block, the tool logs:
- Block number and hash from RPC
- Whether the database block hash matches the RPC block hash
- Exits with error if consistency check fails (missing historical block hashes)

Example output with `--subscribe-persisted-blocks`:
```
INFO reth_stale_direct_db: Starting reth-stale-direct-db
INFO reth_stale_direct_db: RPC WebSocket: ws://127.0.0.1:8546
INFO reth_stale_direct_db: DB Path: "/home/user/.local/share/reth/mainnet"
INFO reth_stale_direct_db: Chain: Mainnet
INFO reth_stale_direct_db: Subscribe persisted blocks: true
INFO reth_stale_direct_db: Database opened successfully
INFO reth_stale_direct_db::monitor: Connecting to WebSocket RPC...
INFO reth_stale_direct_db::monitor: Connected to WebSocket RPC
INFO reth_stale_direct_db::sync: Node is synced
INFO reth_stale_direct_db::monitor: Subscribing to newHeads...
INFO reth_stale_direct_db::monitor: Subscribing to reth_subscribeLatestPersistedBlock...
INFO reth_stale_direct_db::monitor: Subscribed to both streams, waiting...
INFO reth_stale_direct_db::monitor: Header received, buffering block_number=24076371 hash=0x007dc92b... pending=0
INFO reth_stale_direct_db::monitor: Flush triggered by persisted block persisted_block=24076371 blocks_to_flush=1 max_latency_ms=502
INFO reth_stale_direct_db::monitor: New block from RPC block_number=24076371 rpc_block_hash=0x007dc92b... latency_ms=503
INFO reth_stale_direct_db::monitor: Consistency check passed (256 block hashes accessible) block_number=24076371 db_last_block=24076371
INFO reth_stale_direct_db::monitor: Block hash matches block_number=24076371 db_block_hash=0x007dc92b...
INFO reth_stale_direct_db::monitor: Header received, buffering block_number=24076372 hash=0x94ed9823... pending=0
INFO reth_stale_direct_db::monitor: Flush triggered by persisted block persisted_block=24076372 blocks_to_flush=1 max_latency_ms=498
INFO reth_stale_direct_db::monitor: New block from RPC block_number=24076372 rpc_block_hash=0x94ed9823... latency_ms=499
INFO reth_stale_direct_db::monitor: Consistency check passed (256 block hashes accessible) block_number=24076372 db_last_block=24076372
INFO reth_stale_direct_db::monitor: Block hash matches block_number=24076372 db_block_hash=0x94ed9823...
```

## License

MIT OR Apache-2.0

---

This example was created with the help of [Claude Code](https://claude.ai/code).
