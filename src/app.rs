use std::time::Instant;

use crossterm::event::KeyEvent;

use crate::rpc_types::*;

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Dashboard,
    Mempool,
    Network,
    Peers,
}

impl Tab {
    pub const ALL: [Tab; 4] = [Tab::Dashboard, Tab::Mempool, Tab::Network, Tab::Peers];

    pub fn title(self) -> &'static str {
        match self {
            Tab::Dashboard => "Dashboard",
            Tab::Mempool => "Mempool",
            Tab::Network => "Network",
            Tab::Peers => "Peers",
        }
    }

    pub fn next(self) -> Tab {
        match self {
            Tab::Dashboard => Tab::Mempool,
            Tab::Mempool => Tab::Network,
            Tab::Network => Tab::Peers,
            Tab::Peers => Tab::Dashboard,
        }
    }

    pub fn prev(self) -> Tab {
        match self {
            Tab::Dashboard => Tab::Peers,
            Tab::Mempool => Tab::Dashboard,
            Tab::Network => Tab::Mempool,
            Tab::Peers => Tab::Network,
        }
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    #[default]
    Normal,
    Search,
}

pub struct PollResult {
    pub blockchain: Result<BlockchainInfo, String>,
    pub network: Result<NetworkInfo, String>,
    pub mempool: Result<MempoolInfo, String>,
    pub mining: Result<MiningInfo, String>,
    pub peers: Result<Vec<PeerInfo>, String>,
    pub recent_blocks: Option<Vec<BlockStats>>,
}

pub enum SearchResult {
    Mempool { txid: String, entry: MempoolEntry },
    Confirmed { txid: String, tx: RawTransaction },
}

pub enum Event {
    Key(KeyEvent),
    Tick,
    PollComplete(Box<PollResult>),
    SearchComplete(Box<Result<SearchResult, String>>),
}

#[derive(Default)]
pub struct App {
    pub tab: Tab,
    pub input_mode: InputMode,
    pub search_input: String,
    pub should_quit: bool,

    pub blockchain: Option<BlockchainInfo>,
    pub network: Option<NetworkInfo>,
    pub mempool: Option<MempoolInfo>,
    pub mining: Option<MiningInfo>,
    pub peers: Option<Vec<PeerInfo>>,
    pub recent_blocks: Vec<BlockStats>,
    pub last_tip: Option<String>,

    pub search_result: Option<SearchResult>,
    pub search_error: Option<String>,
    pub searching: bool,

    pub rpc_error: Option<String>,
    pub last_update: Option<Instant>,
    pub refreshing: bool,
}

impl App {
    pub fn update(&mut self, event: Event) {
        match event {
            Event::Key(key) => self.handle_key(key),
            Event::Tick => {}
            Event::PollComplete(result) => self.handle_poll(*result),
            Event::SearchComplete(result) => self.handle_search(*result),
        }
    }

    fn handle_poll(&mut self, result: PollResult) {
        self.refreshing = false;
        self.last_update = Some(Instant::now());
        self.rpc_error = None;

        let mut had_error = false;
        match result.blockchain {
            Ok(info) => {
                self.last_tip = Some(info.bestblockhash.clone());
                self.blockchain = Some(info);
            }
            Err(e) => {
                had_error = true;
                self.rpc_error = Some(e);
            }
        }
        match result.network {
            Ok(info) => self.network = Some(info),
            Err(e) if !had_error => {
                had_error = true;
                self.rpc_error = Some(e);
            }
            _ => {}
        }
        match result.mempool {
            Ok(info) => self.mempool = Some(info),
            Err(e) if !had_error => {
                had_error = true;
                self.rpc_error = Some(e);
            }
            _ => {}
        }
        match result.mining {
            Ok(info) => self.mining = Some(info),
            Err(e) if !had_error => {
                had_error = true;
                self.rpc_error = Some(e);
            }
            _ => {}
        }
        match result.peers {
            Ok(info) => self.peers = Some(info),
            Err(e) if !had_error => {
                self.rpc_error = Some(e);
            }
            _ => {}
        }
        if let Some(blocks) = result.recent_blocks {
            self.recent_blocks = blocks;
        }
        let _ = had_error;
    }

    fn handle_search(&mut self, result: Result<SearchResult, String>) {
        self.searching = false;
        match result {
            Ok(sr) => {
                self.search_error = None;
                self.search_result = Some(sr);
            }
            Err(e) => {
                self.search_result = None;
                self.search_error = Some(e);
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        use crossterm::event::{KeyCode, KeyModifiers};

        if (self.search_result.is_some() || self.search_error.is_some()) && key.code == KeyCode::Esc
        {
            self.search_result = None;
            self.search_error = None;
            return;
        }

        match self.input_mode {
            InputMode::Normal => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
                KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
                    self.tab = self.tab.next();
                }
                KeyCode::BackTab => self.tab = self.tab.prev(),
                KeyCode::Left | KeyCode::Char('h') => {
                    self.tab = self.tab.prev();
                }
                KeyCode::Char('/') => {
                    self.input_mode = InputMode::Search;
                    self.search_input.clear();
                }
                _ => {}
            },
            InputMode::Search => match key.code {
                KeyCode::Esc => {
                    self.input_mode = InputMode::Normal;
                    self.search_input.clear();
                }
                KeyCode::Enter => {
                    if !self.search_input.is_empty() {
                        self.searching = true;
                        self.input_mode = InputMode::Normal;
                    }
                }
                KeyCode::Backspace => {
                    self.search_input.pop();
                }
                KeyCode::Char(c) => {
                    if !key.modifiers.contains(KeyModifiers::CONTROL) {
                        self.search_input.push(c);
                    }
                }
                _ => {}
            },
        }
    }
}
