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
cargo run -- --rpc-ws wss://eth-mainnet.example.com --db-path ~/.local/share/reth/mainnet --subscribe-persisted-blocks
```

## CLI Options

| Option | Description                                   | Default |
|--------|-----------------------------------------------|---------|
| `--rpc-ws` | WebSocket RPC URL for block subscriptions     | Required |
| `--db-path` | Path to reth data directory                   | Required |
| `--chain` | Chain to use: `mainnet`, `sepolia`, `holesky` | `mainnet` |
| `--subscribe-persisted-blocks` | Use `reth_subscribeLatestPersistedBlock` instead of standard `eth_subscribe` | `false` |

## Output

On each new block, the tool logs:
- Block number and hash from RPC
- Whether the database block hash matches the RPC block hash
- Exits with error if consistency check fails (missing historical block hashes)

Example output:
```
INFO Starting reth-stale-direct-db
INFO RPC WebSocket: wss://eth-mainnet.example.com
INFO DB Path: "/home/user/.local/share/reth/mainnet"
INFO Chain: Mainnet
INFO Database opened successfully
INFO Connecting to WebSocket RPC...
INFO Connected to WebSocket RPC
INFO Subscribed to new blocks, waiting...
INFO New block from RPC block_number=21234567 rpc_block_hash=0x1234...
INFO ✓ Block hash matches block_number=21234567 hash=0x1234...
```

## License

MIT OR Apache-2.0

---

This example was created with the help of [Claude Code](https://claude.ai/code).
