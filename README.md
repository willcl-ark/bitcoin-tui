# bitcoin-tui

Terminal UI for monitoring a running Bitcoin Core node.

![Dashboard](https://img.shields.io/badge/status-alpha-orange)

## Features

- **Dashboard** — blockchain sync status, network overview, mempool stats with gauges
- **Mempool** — transaction count, fees, memory usage gauge, recent blocks bar chart
- **Network** — connection details, per-network reachability, local addresses
- **Peers** — table with ID, address, type, direction, ping, traffic, sync height, v2 status
- **Wallet** — browse all 56 wallet RPC methods, view help text, type arguments, execute calls and view results
- **Transaction search** — look up any txid in mempool or on-chain via `/`

## Requirements

- Rust 1.85+
- A running Bitcoin Core node with JSON-RPC enabled

## Install

```
cargo install --path .
```

Or run directly:

```
cargo run
```

## Usage

```
bitcoin-tui [OPTIONS]
```

### Connection

By default connects to `127.0.0.1:8332` using cookie auth from `~/.bitcoin/.cookie`.

| Flag | Description |
|------|-------------|
| `--host <HOST>` | RPC host (default: `127.0.0.1`) |
| `--port <PORT>` | RPC port (auto-detected from network if omitted) |
| `--rpccookiefile <PATH>` | Path to `.cookie` file |
| `--rpcuser <USER>` | RPC username |
| `--rpcpassword <PASS>` | RPC password |
| `--interval <SECS>` | Polling interval in seconds (default: `5`) |

### Network selection

| Flag | Port | Cookie path |
|------|------|-------------|
| *(default)* | 8332 | `~/.bitcoin/.cookie` |
| `--testnet` | 18332 | `~/.bitcoin/testnet3/.cookie` |
| `--testnet4` | 48332 | `~/.bitcoin/testnet4/.cookie` |
| `--signet` | 38332 | `~/.bitcoin/signet/.cookie` |
| `--regtest` | 18443 | `~/.bitcoin/regtest/.cookie` |

### Keybindings

| Key | Action |
|-----|--------|
| `Tab` / `l` / `→` | Next tab |
| `Shift+Tab` / `h` / `←` | Previous tab |
| `/` | Search for a transaction |
| `Enter` | Submit search / call wallet method |
| `Esc` | Cancel search / close overlay / quit |
| `j` / `k` | Scroll wallet method list |
| `w` | Set wallet name (Wallet tab) |
| `g` / `G` | Jump to first / last method |
| `q` | Quit |

## Examples

```bash
# Mainnet with default cookie auth
bitcoin-tui

# Regtest
bitcoin-tui --regtest

# Custom auth
bitcoin-tui --rpcuser alice --rpcpassword secret --port 18443

# Faster polling
bitcoin-tui --interval 2
```

## License

MIT
