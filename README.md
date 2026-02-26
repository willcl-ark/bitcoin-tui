# bitcoin-tui

Terminal UI for monitoring and interacting with a running Bitcoin Core node.

![Dashboard](https://img.shields.io/badge/status-alpha-orange)

## Features

### Tabs

- **Dashboard** — blockchain sync progress, network overview, mempool summary with gauges
- **Mempool** — transaction count, fees, memory usage gauge, recent blocks bar chart
- **Network** — connection details, per-network reachability, local addresses
- **Peers** — table with ID, address, type, direction, ping, traffic, sync height, v2 transport status
- **Transactions** — search for any transaction by txid; results show mempool status (fee, size, ancestors/descendants) or confirmed details (confirmations, block age)
- **RPC** — browse all non-wallet RPC methods (blockchain, mempool, mining, network, etc.) with inline help, type arguments, execute calls, and view results
- **Wallet** — browse all 56 wallet RPC methods with inline help, type arguments, execute calls, and view results

### RPC explorer

The RPC tab lists every non-wallet RPC method from the Bitcoin Core API (blockchain, mempool, mining, network, rawtransactions, etc.). Select a method to see its description and parameter documentation. Press `Enter` to call it — methods with no parameters execute immediately, methods with parameters open an argument input. These calls go directly to the node without any wallet context.

### Wallet RPC explorer

The Wallet tab lists every wallet RPC method from the Bitcoin Core API. Select a method to see its description and parameter documentation. Press `Enter` to call it — methods with no parameters execute immediately, methods with parameters open an argument input where you type positional JSON args (e.g. `"*", 6`).

Press `w` to open the wallet selector, which queries `listwallets` from your node and lets you pick which wallet to target. The selected wallet name appears in the method list header and is used for all subsequent calls via the `/wallet/<name>` RPC endpoint.

## Requirements

- Rust 1.85+
- A running Bitcoin Core node with JSON-RPC enabled

## Install

```
cargo install --path .
```

With Nix:

```
nix build
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

The UI uses a two-level focus model: **tab bar** (top-level navigation) and **content** (interacting within a tab).

#### Tab bar (default)

| Key | Action |
|-----|--------|
| `h` / `l` / `←` / `→` | Switch tab |
| `Enter` | Enter tab content |
| `q` / `Esc` | Quit |

#### Content mode

| Key | Action |
|-----|--------|
| `Esc` | Return to tab bar |

#### Transactions tab

| Key | Action |
|-----|--------|
| `/` | Search for a transaction by txid |
| `j` / `k` / `↑` / `↓` | Scroll results |
| `Ctrl+d` / `Ctrl+u` | Page down / up |
| `Esc` | Return to tab bar |

#### RPC / Wallet — methods pane

| Key | Action |
|-----|--------|
| `j` / `k` / `↑` / `↓` | Navigate method list |
| `g` / `G` | Jump to first / last method |
| `/` | Search/filter methods |
| `w` | Open wallet selector (Wallet tab only) |
| `Tab` | Switch to detail pane |
| `Esc` | Return to tab bar |

#### RPC / Wallet — detail pane

| Key | Action |
|-----|--------|
| `Enter` | Call method (opens arg input if method has parameters) |
| `j` / `k` / `↑` / `↓` | Scroll |
| `Ctrl+d` / `Ctrl+u` | Page down / up |
| `/` | Search result text |
| `n` / `N` | Next / previous match |
| `Tab` | Switch to methods pane |
| `Esc` | Return to tab bar |

#### Wallet selector popup

| Key | Action |
|-----|--------|
| `j` / `k` / `↑` / `↓` | Navigate wallet list |
| `Enter` | Select wallet |
| `Esc` | Cancel |

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
