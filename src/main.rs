mod app;
mod format;
mod peers_query;
mod rpc;
mod rpc_types;
mod tabs;
mod ui;
mod wallet_schema;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use crossterm::event::{EventStream, KeyEventKind};
use futures_util::StreamExt;
use tokio::sync::mpsc;
use tokio::time::interval;

use app::{App, Event, PollResult, PsbtRpcAction, PsbtRpcResult, SearchResult, ZmqEntry};
use rpc::RpcClient;

#[derive(Parser)]
#[command(name = "bitcoin-tui", about = "Terminal UI for Bitcoin Core")]
struct Args {
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    #[arg(long)]
    port: Option<u16>,

    #[arg(long)]
    rpccookiefile: Option<String>,

    #[arg(long)]
    rpcuser: Option<String>,

    #[arg(long)]
    rpcpassword: Option<String>,

    #[arg(long, group = "network")]
    testnet: bool,

    #[arg(long, group = "network")]
    testnet4: bool,

    #[arg(long, group = "network")]
    regtest: bool,

    #[arg(long, group = "network")]
    signet: bool,

    #[arg(long, default_value_t = 5)]
    interval: u64,

    #[arg(long, default_value = "127.0.0.1")]
    zmqhost: String,

    #[arg(long)]
    zmqport: Option<u16>,

    #[arg(long)]
    debug: bool,
}

impl Args {
    fn resolve_port(&self) -> u16 {
        if let Some(p) = self.port {
            return p;
        }
        if self.testnet {
            18332
        } else if self.testnet4 {
            48332
        } else if self.regtest {
            18443
        } else if self.signet {
            38332
        } else {
            8332
        }
    }

    fn network_subdir(&self) -> Option<&str> {
        if self.testnet {
            Some("testnet3")
        } else if self.testnet4 {
            Some("testnet4")
        } else if self.regtest {
            Some("regtest")
        } else if self.signet {
            Some("signet")
        } else {
            None
        }
    }

    fn cookie_path(&self) -> Option<PathBuf> {
        self.rpccookiefile
            .as_ref()
            .map(PathBuf::from)
            .or_else(|| Some(rpc::default_cookie_path(self.network_subdir())))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if args.debug {
        use tracing_subscriber::EnvFilter;
        tracing_subscriber::fmt()
            .with_writer(std::io::stderr)
            .with_target(false)
            .with_env_filter(
                EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| EnvFilter::new("bitcoin_tui=debug")),
            )
            .init();
    }

    let rpc_port = args.resolve_port();
    let rpc_url = format!("http://{}:{}", args.host, rpc_port);
    let cookie_path = args.cookie_path();
    let zmq_addr = args
        .zmqport
        .map(|port| format!("tcp://{}:{}", args.zmqhost, port));

    tracing::info!(
        rpc_url,
        cookie_path = ?cookie_path,
        zmq_addr = ?zmq_addr,
        poll_interval = args.interval,
        "starting"
    );

    let rpc = Arc::new(RpcClient::new(
        &args.host,
        rpc_port,
        cookie_path,
        args.rpcuser.as_deref(),
        args.rpcpassword.as_deref(),
    ));

    let mut terminal = ratatui::init();
    let result = run(&mut terminal, rpc, args.interval, zmq_addr).await;
    ratatui::restore();
    result
}

async fn run(
    terminal: &mut ratatui::DefaultTerminal,
    rpc: Arc<RpcClient>,
    poll_interval: u64,
    zmq_addr: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    const EVENT_CHANNEL_CAPACITY: usize = 1024;

    let mut app = App::default();
    let mut reader = EventStream::new();
    let mut tick = interval(Duration::from_millis(250));

    let (tx, mut rx) = mpsc::channel::<Event>(EVENT_CHANNEL_CAPACITY);

    spawn_polling(rpc.clone(), tx.clone(), poll_interval);

    if let Some(addr) = zmq_addr {
        app.zmq.enabled = true;
        spawn_zmq(addr, tx.clone());
    }

    loop {
        terminal.draw(|frame| ui::render(&app, frame))?;

        if app.transactions.searching {
            app.transactions.searching = false;
            app.transactions.request_seq = app.transactions.request_seq.wrapping_add(1);
            let request_id = app.transactions.request_seq;
            app.transactions.in_flight_request = Some(request_id);
            let txid = app.transactions.search_input.clone();
            let rpc = rpc.clone();
            let tx = tx.clone();
            tokio::spawn(async move {
                let result = search_tx(&rpc, &txid).await;
                let _ = tx
                    .send(Event::SearchComplete(request_id, Box::new(result)))
                    .await;
            });
        }

        if app.wallet.fetching_wallets {
            app.wallet.fetching_wallets = false;
            let rpc = rpc.clone();
            let tx = tx.clone();
            tokio::spawn(async move {
                let result = rpc
                    .call_raw("listwallets", serde_json::json!([]), None)
                    .await
                    .and_then(|v| {
                        serde_json::from_value::<Vec<String>>(v)
                            .map_err(|e| format!("Failed to parse listwallets: {}", e))
                    });
                let _ = tx.send(Event::WalletListComplete(Box::new(result))).await;
            });
        }

        if app.wallet.browser.calling {
            app.wallet.browser.calling = false;
            app.wallet.browser.request_seq = app.wallet.browser.request_seq.wrapping_add(1);
            let request_id = app.wallet.browser.request_seq;
            app.wallet.browser.in_flight_request = Some(request_id);
            let method = app.wallet.browser.methods[app.wallet.browser.selected]
                .name
                .clone();
            let arg_text = app.wallet.browser.arg_input.clone();
            let wallet_name = app.wallet.wallet_name.clone();
            let rpc = rpc.clone();
            let tx = tx.clone();
            tokio::spawn(async move {
                let params = parse_args(&arg_text);
                let wallet = if wallet_name.is_empty() {
                    None
                } else {
                    Some(wallet_name.as_str())
                };
                let result = match params {
                    Ok(p) => rpc.call_raw(&method, p, wallet).await.map(|v| {
                        serde_json::to_string_pretty(&v).unwrap_or_else(|_| v.to_string())
                    }),
                    Err(e) => Err(e),
                };
                let _ = tx
                    .send(Event::WalletRpcComplete(request_id, Box::new(result)))
                    .await;
            });
        }

        if app.rpc.calling {
            app.rpc.calling = false;
            app.rpc.request_seq = app.rpc.request_seq.wrapping_add(1);
            let request_id = app.rpc.request_seq;
            app.rpc.in_flight_request = Some(request_id);
            let method = app.rpc.methods[app.rpc.selected].name.clone();
            let arg_text = app.rpc.arg_input.clone();
            let rpc = rpc.clone();
            let tx = tx.clone();
            tokio::spawn(async move {
                let params = parse_args(&arg_text);
                let result = match params {
                    Ok(p) => rpc.call_raw(&method, p, None).await.map(|v| {
                        serde_json::to_string_pretty(&v).unwrap_or_else(|_| v.to_string())
                    }),
                    Err(e) => Err(e),
                };
                let _ = tx.send(Event::RpcComplete(request_id, Box::new(result))).await;
            });
        }

        if app.psbt.in_flight_request.is_none() && let Some(action) = app.psbt.rpc_in_flight.take() {
            app.psbt.request_seq = app.psbt.request_seq.wrapping_add(1);
            let request_id = app.psbt.request_seq;
            app.psbt.in_flight_request = Some(request_id);
            app.psbt.running_action = Some(action);
            let psbt = app.psbt.psbt.trim().to_string();
            let wallet_name = app.wallet.wallet_name.clone();
            let rpc = rpc.clone();
            let tx = tx.clone();
            tokio::spawn(async move {
                let result = run_psbt_action(&rpc, action, &psbt, &wallet_name).await;
                let _ = tx
                    .send(Event::PsbtRpcComplete(request_id, Box::new(result)))
                    .await;
            });
        }

        if let Some(block_hash) = app.zmq.block_lookup.take() {
            let rpc = rpc.clone();
            let tx = tx.clone();
            tokio::spawn(async move {
                let result = rpc
                    .call_raw("getblock", serde_json::json!([block_hash, 1]), None)
                    .await
                    .map(|v| serde_json::to_string_pretty(&v).unwrap_or_else(|_| v.to_string()));
                let _ = tx.send(Event::ZmqBlockComplete(Box::new(result))).await;
            });
        }

        tokio::select! {
            _ = tick.tick() => {
                app.update(Event::Tick);
            }
            event = reader.next() => {
                if let Some(Ok(crossterm::event::Event::Key(key))) = event
                    && key.kind == KeyEventKind::Press
                {
                    app.update(Event::Key(key));
                }
            }
            event = rx.recv() => {
                if let Some(event) = &event {
                    tracing::trace!(event = ?std::mem::discriminant(event), "channel recv");
                }
                if let Some(event) = event {
                    app.update(event);
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

fn spawn_polling(rpc: Arc<RpcClient>, tx: mpsc::Sender<Event>, interval_secs: u64) {
    tokio::spawn(async move {
        const RECENT_BLOCK_HISTORY: u64 = 72;
        const SLOW_RPC_REFRESH_POLLS: u64 = 6;
        let mut last_tip: Option<String> = None;
        let mut last_height: Option<u64> = None;
        let mut polls_since_slow_refresh: u64 = SLOW_RPC_REFRESH_POLLS;
        let mut cached_recent_blocks: Vec<crate::rpc_types::BlockStats> = Vec::new();
        let mut cached_mining: Option<crate::rpc_types::MiningInfo> = None;
        let mut cached_chaintips: Option<Vec<crate::rpc_types::ChainTip>> = None;
        let mut tip_pool_cache: std::collections::HashMap<String, Option<String>> =
            std::collections::HashMap::new();
        loop {
            tracing::debug!("rpc poll starting");
            let (blockchain, network, mempool, peers, nettotals) = tokio::join!(
                rpc.get_blockchain_info(),
                rpc.get_network_info(),
                rpc.get_mempool_info(),
                rpc.get_peer_info(),
                rpc.get_net_totals(),
            );

            let tip_changed = match (&blockchain, &last_tip) {
                (Ok(info), Some(old_tip)) => info.bestblockhash != *old_tip,
                (Ok(_), None) => true,
                _ => false,
            };

            let needs_slow_refresh = tip_changed
                || polls_since_slow_refresh >= SLOW_RPC_REFRESH_POLLS
                || cached_mining.is_none()
                || cached_chaintips.is_none();

            let (mining, chaintips) = if needs_slow_refresh {
                let (mining, chaintips) = tokio::join!(rpc.get_mining_info(), rpc.get_chain_tips(),);
                if let Ok(info) = &mining {
                    cached_mining = Some(info.clone());
                }
                if let Ok(tips) = &chaintips {
                    cached_chaintips = Some(tips.clone());
                }
                polls_since_slow_refresh = 0;
                (mining, chaintips)
            } else {
                polls_since_slow_refresh = polls_since_slow_refresh.saturating_add(1);
                tracing::trace!("reusing cached mining/chaintips");
                (
                    cached_mining
                        .clone()
                        .ok_or_else(|| "Mining info unavailable".to_string()),
                    cached_chaintips
                        .clone()
                        .ok_or_else(|| "Chain tips unavailable".to_string()),
                )
            };

            tracing::debug!(tip_changed, needs_slow_refresh, "rpc poll complete");
            let tip_info = if tip_changed {
                blockchain
                    .as_ref()
                    .ok()
                    .map(|info| (info.bestblockhash.clone(), info.blocks))
            } else {
                None
            };

            let tips_to_enrich = chaintips.as_ref().ok().cloned();

            let result = PollResult {
                blockchain,
                network,
                mempool,
                mining,
                peers,
                nettotals,
                chaintips,
            };

            if tx.send(Event::PollComplete(Box::new(result))).await.is_err() {
                break;
            }

            if let Some(mut tips) = tips_to_enrich {
                let mut changed = false;
                for tip in &mut tips {
                    if let Some(cached) = tip_pool_cache.get(&tip.hash) {
                        tip.pool = cached.clone();
                    } else {
                        let pool = get_block_pool_by_hash(&rpc, &tip.hash).await;
                        tip_pool_cache.insert(tip.hash.clone(), pool.clone());
                        tip.pool = pool;
                        changed = true;
                    }
                }
                if changed {
                    let _ = tx.send(Event::ChainTipsEnriched(tips)).await;
                }
            }

            if let Some((tip_hash, height)) = tip_info {
                    let is_cold_start = last_height.is_none();
                    let mut updated = cached_recent_blocks.clone();
                    let mut start_height = height.saturating_sub(RECENT_BLOCK_HISTORY - 1);

                    if let Some(prev_height) = last_height {
                        let delta = height.saturating_sub(prev_height);
                        if height > prev_height && delta <= RECENT_BLOCK_HISTORY {
                            start_height = prev_height + 1;
                        } else {
                            updated.clear();
                        }
                    } else {
                        updated.clear();
                    }

                    if is_cold_start {
                        // Fetch from tip downward, sending incremental updates
                        for h in (start_height..=height).rev() {
                            if let Ok(mut stats) = rpc.get_block_stats(h).await {
                                stats.pool = get_block_pool(&rpc, h).await;
                                updated.push(stats);
                                updated.sort_by_key(|b| b.height);
                                let snapshot = updated.clone();
                                let _ = tx.send(Event::RecentBlocksComplete(snapshot)).await;
                            }
                        }
                    } else {
                        for h in start_height..=height {
                            if let Ok(mut stats) = rpc.get_block_stats(h).await {
                                stats.pool = get_block_pool(&rpc, h).await;
                                updated.push(stats);
                            }
                        }
                    }

                    if updated.len() > RECENT_BLOCK_HISTORY as usize {
                        let keep_from = updated.len() - RECENT_BLOCK_HISTORY as usize;
                        updated.drain(0..keep_from);
                    }

                    if !updated.is_empty() {
                        cached_recent_blocks = updated.clone();
                        let _ = tx.send(Event::RecentBlocksComplete(updated)).await;
                    }

                    last_tip = Some(tip_hash);
                    last_height = Some(height);
            }

            tokio::time::sleep(Duration::from_secs(interval_secs)).await;
        }
    });
}

async fn get_block_pool(rpc: &rpc::RpcClient, height: u64) -> Option<String> {
    let hash = rpc.get_block_hash(height).await.ok()?;
    get_block_pool_by_hash(rpc, &hash).await
}

async fn get_block_pool_by_hash(rpc: &rpc::RpcClient, hash: &str) -> Option<String> {
    let block: serde_json::Value = rpc
        .call_raw("getblock", serde_json::json!([hash, 1]), None)
        .await
        .ok()?;
    let txid = block["tx"][0].as_str()?;
    let tx = rpc.get_raw_transaction(txid).await.ok()?;
    let coinbase_hex = tx.vin.first()?.coinbase.as_ref()?;
    extract_pool_name(coinbase_hex)
}

fn extract_pool_name(coinbase_hex: &str) -> Option<String> {
    let bytes: Vec<u8> = (0..coinbase_hex.len())
        .step_by(2)
        .filter_map(|i| u8::from_str_radix(coinbase_hex.get(i..i + 2)?, 16).ok())
        .collect();

    // Try /Name/ pattern first (most common pool tag convention)
    let mut last_match = None;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'/' {
            if let Some(end) = bytes[i + 1..].iter().position(|&b| b == b'/') {
                let name = &bytes[i + 1..i + 1 + end];
                if !name.is_empty() && name.iter().all(|&b| b.is_ascii_graphic() || b == b' ') {
                    last_match = Some(String::from_utf8_lossy(name).into_owned());
                }
                i += end + 2;
                continue;
            }
        }
        i += 1;
    }
    if last_match.is_some() {
        return last_match;
    }

    // Fallback: longest printable ASCII run (e.g. "Powered by Luxor Tech")
    let mut best = "";
    let mut start = None;
    for (i, &b) in bytes.iter().enumerate() {
        if b >= 0x20 && b <= 0x7e {
            if start.is_none() {
                start = Some(i);
            }
        } else if let Some(s) = start.take() {
            if i - s > best.len() {
                best = std::str::from_utf8(&bytes[s..i]).unwrap_or("");
            }
        }
    }
    if let Some(s) = start {
        if bytes.len() - s > best.len() {
            best = std::str::from_utf8(&bytes[s..]).unwrap_or("");
        }
    }

    if best.len() >= 4 {
        Some(best.trim().to_string())
    } else {
        None
    }
}

fn spawn_zmq(addr: String, tx: mpsc::Sender<Event>) {
    use zeromq::{Socket, SocketRecv, SubSocket, ZmqMessage};

    tokio::spawn(async move {
        let mut socket = SubSocket::new();
        // Subscribe to all topics and filter in-process so topic prefix mismatches don't
        // silently suppress notifications.
        if let Err(e) = socket.subscribe("").await {
            tracing::error!(error = %e, "zmq subscribe failed");
            let _ = tx.send(Event::ZmqError(format!("subscribe all: {}", e))).await;
            return;
        }
        tracing::debug!("zmq subscribed to all topics");
        tracing::info!(addr, "zmq connecting");
        if let Err(e) = socket.connect(&addr).await {
            tracing::error!(addr, error = %e, "zmq connect failed");
            let _ = tx.send(Event::ZmqError(format!("connect {}: {}", addr, e))).await;
            return;
        }

        tracing::debug!("zmq waiting for messages");
        loop {
            let msg: ZmqMessage = match socket.recv().await {
                Ok(msg) => msg,
                Err(e) => {
                    tracing::error!(error = %e, "zmq recv failed");
                    let _ = tx.send(Event::ZmqError(format!("recv: {}", e))).await;
                    break;
                }
            };
            let frames: Vec<_> = msg.into_vec();
            if frames.len() < 2 {
                tracing::warn!(frames = frames.len(), "zmq: skipping message with unexpected frame count");
                continue;
            }
            let topic = String::from_utf8_lossy(&frames[0]).trim_end_matches('\0').to_string();
            if topic != "hashtx" && topic != "hashblock" {
                continue;
            }
            let hash_bytes = &frames[1];
            let hash = hash_bytes
                .iter()
                .rev()
                .map(|b| format!("{:02x}", b))
                .collect::<String>();

            tracing::debug!(topic, hash, "zmq recv");

            if tx
                .send(Event::ZmqMessage(Box::new(ZmqEntry { topic, hash })))
                .await
                .is_err()
            {
                break;
            }
        }
    });
}

fn parse_args(input: &str) -> Result<serde_json::Value, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(serde_json::json!([]));
    }
    let wrapped = format!("[{}]", trimmed);
    serde_json::from_str(&wrapped).map_err(|e| format!("Invalid args: {}", e))
}

async fn search_tx(rpc: &RpcClient, txid: &str) -> Result<SearchResult, String> {
    for candidate in txid_candidates(txid) {
        tracing::debug!(requested = txid, candidate, "searching for tx");
        if let Ok(entry) = rpc.get_mempool_entry(&candidate).await {
            tracing::debug!(txid = candidate, "found in mempool");
            let decoded = decode_tx_for_display(rpc, &candidate).await;
            return Ok(SearchResult::Mempool {
                txid: candidate,
                entry,
                decoded,
            });
        }
        if let Ok(tx) = rpc.get_raw_transaction(&candidate).await {
            tracing::debug!(txid = candidate, "found confirmed");
            let decoded = decode_tx_for_display(rpc, &candidate).await;
            return Ok(SearchResult::Confirmed {
                txid: candidate,
                tx,
                decoded,
            });
        }
    }
    tracing::debug!(txid, "tx not found");
    Err("Transaction not found".to_string())
}

fn txid_candidates(txid: &str) -> Vec<String> {
    let trimmed = txid.trim();
    let mut out = vec![trimmed.to_string()];
    if let Some(reversed) = reverse_32byte_hex(trimmed)
        && reversed != trimmed
    {
        out.push(reversed);
    }
    out
}

fn reverse_32byte_hex(s: &str) -> Option<String> {
    if s.len() != 64 || !s.as_bytes().iter().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }

    let mut out = String::with_capacity(64);
    for i in (0..64).step_by(2).rev() {
        out.push_str(&s[i..i + 2]);
    }
    Some(out)
}

async fn decode_tx_for_display(rpc: &RpcClient, txid: &str) -> Option<String> {
    let hex = match rpc.get_raw_transaction_hex(txid).await {
        Ok(hex) => hex,
        Err(e) => {
            tracing::debug!(txid, error = %e, "getrawtransaction hex failed");
            return None;
        }
    };
    match rpc.decode_raw_transaction(&hex).await {
        Ok(decoded) => Some(
            serde_json::to_string_pretty(&decoded).unwrap_or_else(|_| decoded.to_string()),
        ),
        Err(e) => {
            tracing::debug!(txid, error = %e, "decoderawtransaction failed");
            None
        }
    }
}

async fn run_psbt_action(
    rpc: &RpcClient,
    action: PsbtRpcAction,
    psbt: &str,
    wallet_name: &str,
) -> Result<PsbtRpcResult, String> {
    if psbt.is_empty() {
        return Err("No PSBT loaded".to_string());
    }

    let wallet = if wallet_name.is_empty() {
        None
    } else {
        Some(wallet_name)
    };

    let (method, params, wallet_ctx) = match action {
        PsbtRpcAction::Decode => ("decodepsbt", serde_json::json!([psbt]), None),
        PsbtRpcAction::Analyze => ("analyzepsbt", serde_json::json!([psbt]), None),
        PsbtRpcAction::WalletProcess => (
            "walletprocesspsbt",
            serde_json::json!([psbt, true, "DEFAULT", true, false]),
            wallet,
        ),
        PsbtRpcAction::Finalize => ("finalizepsbt", serde_json::json!([psbt, false]), None),
        PsbtRpcAction::UtxoUpdate => ("utxoupdatepsbt", serde_json::json!([psbt]), None),
    };

    let value = rpc.call_raw(method, params, wallet_ctx).await?;
    let output_json = serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string());
    let updated_psbt = match action {
        PsbtRpcAction::UtxoUpdate => value.as_str().map(str::to_string),
        _ => value
            .get("psbt")
            .and_then(|v| v.as_str())
            .map(str::to_string),
    };

    Ok(PsbtRpcResult {
        action,
        output_json,
        updated_psbt,
    })
}
