mod app;
mod format;
mod rpc;
mod rpc_types;
mod tabs;
mod ui;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use crossterm::event::{EventStream, KeyEventKind};
use futures_util::StreamExt;
use tokio::sync::mpsc;
use tokio::time::interval;

use app::{App, Event, PollResult, SearchResult};
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

    let rpc = Arc::new(RpcClient::new(
        &args.host,
        args.resolve_port(),
        args.cookie_path(),
        args.rpcuser.as_deref(),
        args.rpcpassword.as_deref(),
    ));

    let mut terminal = ratatui::init();
    let result = run(&mut terminal, rpc, args.interval).await;
    ratatui::restore();
    result
}

async fn run(
    terminal: &mut ratatui::DefaultTerminal,
    rpc: Arc<RpcClient>,
    poll_interval: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::default();
    let mut reader = EventStream::new();
    let mut tick = interval(Duration::from_secs(1));

    let (tx, mut rx) = mpsc::unbounded_channel::<Event>();

    spawn_polling(rpc.clone(), tx.clone(), poll_interval);

    loop {
        terminal.draw(|frame| ui::render(&app, frame))?;

        if app.searching {
            app.searching = false;
            let txid = app.search_input.clone();
            let rpc = rpc.clone();
            let tx = tx.clone();
            tokio::spawn(async move {
                let result = search_tx(&rpc, &txid).await;
                let _ = tx.send(Event::SearchComplete(Box::new(result)));
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
            let (blockchain, network, mempool, mining, peers) = tokio::join!(
                rpc.get_blockchain_info(),
                rpc.get_network_info(),
                rpc.get_mempool_info(),
                rpc.get_mining_info(),
                rpc.get_peer_info(),
            );

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

async fn search_tx(rpc: &RpcClient, txid: &str) -> Result<SearchResult, String> {
    if let Ok(entry) = rpc.get_mempool_entry(txid).await {
        return Ok(SearchResult::Mempool {
            txid: txid.to_string(),
            entry,
        });
    }
    if let Ok(tx) = rpc.get_raw_transaction(txid).await {
        return Ok(SearchResult::Confirmed {
            txid: txid.to_string(),
            tx,
        });
    }
    Err("Transaction not found".to_string())
}
