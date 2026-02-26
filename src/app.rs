use std::time::Instant;

use crossterm::event::KeyEvent;
use ratatui::widgets::ListState;

use crate::rpc_types::*;
use crate::wallet_schema::{RpcMethod, load_wallet_methods};

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Dashboard,
    Mempool,
    Network,
    Peers,
    Wallet,
}

impl Tab {
    pub const ALL: [Tab; 5] = [
        Tab::Dashboard,
        Tab::Mempool,
        Tab::Network,
        Tab::Peers,
        Tab::Wallet,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Tab::Dashboard => "Dashboard",
            Tab::Mempool => "Mempool",
            Tab::Network => "Network",
            Tab::Peers => "Peers",
            Tab::Wallet => "Wallet",
        }
    }

    pub fn next(self) -> Tab {
        match self {
            Tab::Dashboard => Tab::Mempool,
            Tab::Mempool => Tab::Network,
            Tab::Network => Tab::Peers,
            Tab::Peers => Tab::Wallet,
            Tab::Wallet => Tab::Dashboard,
        }
    }

    pub fn prev(self) -> Tab {
        match self {
            Tab::Dashboard => Tab::Wallet,
            Tab::Mempool => Tab::Dashboard,
            Tab::Network => Tab::Mempool,
            Tab::Peers => Tab::Network,
            Tab::Wallet => Tab::Peers,
        }
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    #[default]
    Normal,
    Search,
    WalletArg,
    WalletPicker,
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
    WalletRpcComplete(Box<Result<String, String>>),
    WalletListComplete(Box<Result<Vec<String>, String>>),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WalletFocus {
    List,
    Args,
}

pub struct WalletTab {
    pub methods: Vec<RpcMethod>,
    pub selected: usize,
    pub list_state: ListState,
    pub focus: WalletFocus,
    pub arg_input: String,
    pub result: Option<String>,
    pub error: Option<String>,
    pub calling: bool,
    pub result_scroll: u16,
    pub wallet_name: String,
    pub wallets: Vec<String>,
    pub picker_index: usize,
    pub fetching_wallets: bool,
}

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

    pub wallet: WalletTab,
}

impl Default for App {
    fn default() -> Self {
        let methods = load_wallet_methods();
        let mut list_state = ListState::default();
        if !methods.is_empty() {
            list_state.select(Some(0));
        }
        App {
            tab: Tab::default(),
            input_mode: InputMode::default(),
            search_input: String::new(),
            should_quit: false,
            blockchain: None,
            network: None,
            mempool: None,
            mining: None,
            peers: None,
            recent_blocks: Vec::new(),
            last_tip: None,
            search_result: None,
            search_error: None,
            searching: false,
            rpc_error: None,
            last_update: None,
            refreshing: false,
            wallet: WalletTab {
                methods,
                selected: 0,
                list_state,
                focus: WalletFocus::List,
                arg_input: String::new(),
                result: None,
                error: None,
                calling: false,
                result_scroll: 0,
                wallet_name: String::new(),
                wallets: Vec::new(),
                picker_index: 0,
                fetching_wallets: false,
            },
        }
    }
}

impl App {
    pub fn update(&mut self, event: Event) {
        match event {
            Event::Key(key) => self.handle_key(key),
            Event::Tick => {}
            Event::PollComplete(result) => self.handle_poll(*result),
            Event::SearchComplete(result) => self.handle_search(*result),
            Event::WalletListComplete(result) => {
                self.wallet.fetching_wallets = false;
                match *result {
                    Ok(list) => {
                        self.wallet.wallets = list;
                        self.wallet.picker_index = self
                            .wallet
                            .wallets
                            .iter()
                            .position(|w| *w == self.wallet.wallet_name)
                            .unwrap_or(0);
                        self.input_mode = InputMode::WalletPicker;
                    }
                    Err(e) => {
                        self.wallet.error = Some(format!("listwallets failed: {}", e));
                    }
                }
            }
            Event::WalletRpcComplete(result) => {
                self.wallet.calling = false;
                match *result {
                    Ok(json) => {
                        self.wallet.error = None;
                        self.wallet.result = Some(json);
                        self.wallet.result_scroll = 0;
                    }
                    Err(e) => {
                        self.wallet.result = None;
                        self.wallet.error = Some(e);
                    }
                }
            }
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
            InputMode::Normal => {
                if self.tab == Tab::Wallet {
                    match key.code {
                        KeyCode::Char('j') | KeyCode::Down => {
                            let len = self.wallet.methods.len();
                            if len > 0 {
                                self.wallet.selected = (self.wallet.selected + 1) % len;
                                self.wallet.list_state.select(Some(self.wallet.selected));
                                self.wallet.result = None;
                                self.wallet.error = None;
                                self.wallet.arg_input.clear();
                                self.wallet.result_scroll = 0;
                            }
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            let len = self.wallet.methods.len();
                            if len > 0 {
                                self.wallet.selected = (self.wallet.selected + len - 1) % len;
                                self.wallet.list_state.select(Some(self.wallet.selected));
                                self.wallet.result = None;
                                self.wallet.error = None;
                                self.wallet.arg_input.clear();
                                self.wallet.result_scroll = 0;
                            }
                        }
                        KeyCode::Enter => {
                            let method = &self.wallet.methods[self.wallet.selected];
                            if method.params.is_empty() {
                                self.wallet.calling = true;
                            } else {
                                self.input_mode = InputMode::WalletArg;
                                self.wallet.focus = WalletFocus::Args;
                            }
                        }
                        KeyCode::Char('w') => {
                            self.wallet.fetching_wallets = true;
                        }
                        KeyCode::Char('G') => {
                            let len = self.wallet.methods.len();
                            if len > 0 {
                                self.wallet.selected = len - 1;
                                self.wallet.list_state.select(Some(self.wallet.selected));
                            }
                        }
                        KeyCode::Char('g') => {
                            if !self.wallet.methods.is_empty() {
                                self.wallet.selected = 0;
                                self.wallet.list_state.select(Some(0));
                            }
                        }
                        KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
                        KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
                            self.tab = self.tab.next();
                        }
                        KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => {
                            self.tab = self.tab.prev();
                        }
                        KeyCode::Char('/') => {
                            self.input_mode = InputMode::Search;
                            self.search_input.clear();
                        }
                        _ => {}
                    }
                } else {
                    match key.code {
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
                    }
                }
            }
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
            InputMode::WalletArg => match key.code {
                KeyCode::Esc => {
                    self.input_mode = InputMode::Normal;
                    self.wallet.focus = WalletFocus::List;
                    self.wallet.arg_input.clear();
                }
                KeyCode::Enter => {
                    self.wallet.calling = true;
                    self.wallet.focus = WalletFocus::List;
                    self.input_mode = InputMode::Normal;
                }
                KeyCode::Backspace => {
                    self.wallet.arg_input.pop();
                }
                KeyCode::Char(c) => {
                    if !key.modifiers.contains(KeyModifiers::CONTROL) {
                        self.wallet.arg_input.push(c);
                    }
                }
                _ => {}
            },
            InputMode::WalletPicker => match key.code {
                KeyCode::Esc => {
                    self.input_mode = InputMode::Normal;
                }
                KeyCode::Enter => {
                    if !self.wallet.wallets.is_empty() {
                        self.wallet.wallet_name =
                            self.wallet.wallets[self.wallet.picker_index].clone();
                    }
                    self.input_mode = InputMode::Normal;
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    let len = self.wallet.wallets.len();
                    if len > 0 {
                        self.wallet.picker_index = (self.wallet.picker_index + 1) % len;
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    let len = self.wallet.wallets.len();
                    if len > 0 {
                        self.wallet.picker_index = (self.wallet.picker_index + len - 1) % len;
                    }
                }
                _ => {}
            },
        }
    }
}
