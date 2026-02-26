mod app;
mod format;
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

use app::{App, Event, PollResult, SearchResult, ZmqEntry};
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
    let mut app = App::default();
    let mut reader = EventStream::new();
    let mut tick = interval(Duration::from_secs(1));

    let (tx, mut rx) = mpsc::unbounded_channel::<Event>();

    spawn_polling(rpc.clone(), tx.clone(), poll_interval);

    if let Some(addr) = zmq_addr {
        app.zmq.enabled = true;
        spawn_zmq(addr, tx.clone());
    }

    loop {
        terminal.draw(|frame| ui::render(&app, frame))?;

        if app.transactions.searching {
            app.transactions.searching = false;
            let txid = app.transactions.search_input.clone();
            let rpc = rpc.clone();
            let tx = tx.clone();
            tokio::spawn(async move {
                let result = search_tx(&rpc, &txid).await;
                let _ = tx.send(Event::SearchComplete(Box::new(result)));
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
                let _ = tx.send(Event::WalletListComplete(Box::new(result)));
            });
        }

        if app.wallet.browser.calling {
            app.wallet.browser.calling = false;
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
                let _ = tx.send(Event::WalletRpcComplete(Box::new(result)));
            });
        }

        if app.rpc.calling {
            app.rpc.calling = false;
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
                let _ = tx.send(Event::RpcComplete(Box::new(result)));
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

fn spawn_polling(rpc: Arc<RpcClient>, tx: mpsc::UnboundedSender<Event>, interval_secs: u64) {
    tokio::spawn(async move {
        let mut last_tip: Option<String> = None;
        loop {
            tracing::debug!("rpc poll starting");
            let (blockchain, network, mempool, mining, peers) = tokio::join!(
                rpc.get_blockchain_info(),
                rpc.get_network_info(),
                rpc.get_mempool_info(),
                rpc.get_mining_info(),
                rpc.get_peer_info(),
            );
            tracing::debug!("rpc poll complete");

            let tip_changed = match (&blockchain, &last_tip) {
                (Ok(info), Some(old_tip)) => info.bestblockhash != *old_tip,
                (Ok(_), None) => true,
                _ => false,
            };

            let recent_blocks = if tip_changed {
                if let Ok(ref info) = blockchain {
                    last_tip = Some(info.bestblockhash.clone());
                    let height = info.blocks;
                    let mut blocks = Vec::new();
                    for h in height.saturating_sub(5)..=height {
                        if let Ok(stats) = rpc.get_block_stats(h).await {
                            blocks.push(stats);
                        }
                    }
                    Some(blocks)
                } else {
                    None
                }
            } else {
                None
            };

            let result = PollResult {
                blockchain,
                network,
                mempool,
                mining,
                peers,
                recent_blocks,
            };

            if tx.send(Event::PollComplete(Box::new(result))).is_err() {
                break;
            }

            tokio::time::sleep(Duration::from_secs(interval_secs)).await;
        }
    });
}

fn spawn_zmq(addr: String, tx: mpsc::UnboundedSender<Event>) {
    use zeromq::{Socket, SocketRecv, SubSocket, ZmqMessage};

    tokio::spawn(async move {
        let mut socket = SubSocket::new();
        // Subscribe to all topics and filter in-process so topic prefix mismatches don't
        // silently suppress notifications.
        if let Err(e) = socket.subscribe("").await {
            tracing::error!(error = %e, "zmq subscribe failed");
            let _ = tx.send(Event::ZmqError(format!("subscribe all: {}", e)));
            return;
        }
        tracing::debug!("zmq subscribed to all topics");
        tracing::info!(addr, "zmq connecting");
        if let Err(e) = socket.connect(&addr).await {
            tracing::error!(addr, error = %e, "zmq connect failed");
            let _ = tx.send(Event::ZmqError(format!("connect {}: {}", addr, e)));
            return;
        }

        tracing::debug!("zmq waiting for messages");
        loop {
            let msg: ZmqMessage = match socket.recv().await {
                Ok(msg) => msg,
                Err(e) => {
                    tracing::error!(error = %e, "zmq recv failed");
                    let _ = tx.send(Event::ZmqError(format!("recv: {}", e)));
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
            let mut reversed = hash_bytes.to_vec();
            reversed.reverse();
            let hash = reversed
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>();

            tracing::debug!(topic, hash, "zmq recv");

            if tx
                .send(Event::ZmqMessage(Box::new(ZmqEntry { topic, hash })))
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
    tracing::debug!(txid, "searching for tx");
    if let Ok(entry) = rpc.get_mempool_entry(txid).await {
        tracing::debug!(txid, "found in mempool");
        return Ok(SearchResult::Mempool {
            txid: txid.to_string(),
            entry,
        });
    }
    if let Ok(tx) = rpc.get_raw_transaction(txid).await {
        tracing::debug!(txid, "found confirmed");
        return Ok(SearchResult::Confirmed {
            txid: txid.to_string(),
            tx,
        });
    }
    tracing::debug!(txid, "tx not found");
    Err("Transaction not found".to_string())
}
