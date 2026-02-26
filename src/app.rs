use std::time::Instant;

use crossterm::event::KeyEvent;
use ratatui::widgets::ListState;

use crate::rpc_types::*;
use crate::wallet_schema::{RpcMethod, load_non_wallet_methods, load_wallet_methods};

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Dashboard,
    Mempool,
    Network,
    Peers,
    Transactions,
    Rpc,
    Wallet,
}

impl Tab {
    pub const ALL: [Tab; 7] = [
        Tab::Dashboard,
        Tab::Mempool,
        Tab::Network,
        Tab::Peers,
        Tab::Rpc,
        Tab::Wallet,
        Tab::Transactions,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Tab::Dashboard => "Dashboard",
            Tab::Mempool => "Mempool",
            Tab::Network => "Network",
            Tab::Peers => "Peers",
            Tab::Rpc => "RPC",
            Tab::Wallet => "Wallet",
            Tab::Transactions => "Transactions",
        }
    }

    pub fn next(self) -> Tab {
        match self {
            Tab::Dashboard => Tab::Mempool,
            Tab::Mempool => Tab::Network,
            Tab::Network => Tab::Peers,
            Tab::Peers => Tab::Rpc,
            Tab::Rpc => Tab::Wallet,
            Tab::Wallet => Tab::Transactions,
            Tab::Transactions => Tab::Dashboard,
        }
    }

    pub fn prev(self) -> Tab {
        match self {
            Tab::Dashboard => Tab::Transactions,
            Tab::Mempool => Tab::Dashboard,
            Tab::Network => Tab::Mempool,
            Tab::Peers => Tab::Network,
            Tab::Rpc => Tab::Peers,
            Tab::Wallet => Tab::Rpc,
            Tab::Transactions => Tab::Wallet,
        }
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    #[default]
    TabBar,
    Content,
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    #[default]
    Normal,
    TxSearch,
    ArgInput,
    WalletPicker,
    MethodSearch,
    DetailSearch,
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
    RpcComplete(Box<Result<String, String>>),
    WalletListComplete(Box<Result<Vec<String>, String>>),
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum BrowserPane {
    #[default]
    Methods,
    Detail,
}

pub struct MethodBrowser {
    pub methods: Vec<RpcMethod>,
    pub selected: usize,
    pub list_state: ListState,
    pub pane: BrowserPane,
    pub arg_input: String,
    pub result: Option<String>,
    pub error: Option<String>,
    pub calling: bool,
    pub result_scroll: u16,
    pub editing_args: bool,
    pub method_search: String,
    pub filtered_indices: Vec<usize>,
    pub filtered_selected: usize,
    pub detail_search: String,
    pub detail_matches: Vec<u16>,
    pub detail_match_index: usize,
}

impl MethodBrowser {
    pub fn new(methods: Vec<RpcMethod>) -> Self {
        let len = methods.len();
        let mut list_state = ListState::default();
        if len > 0 {
            list_state.select(Some(0));
        }
        let filtered_indices: Vec<usize> = (0..len).collect();
        MethodBrowser {
            methods,
            selected: 0,
            list_state,
            pane: BrowserPane::default(),
            arg_input: String::new(),
            result: None,
            error: None,
            calling: false,
            result_scroll: 0,
            editing_args: false,
            method_search: String::new(),
            filtered_indices,
            filtered_selected: 0,
            detail_search: String::new(),
            detail_matches: Vec::new(),
            detail_match_index: 0,
        }
    }

    pub fn update_method_filter(&mut self) {
        let query = self.method_search.to_lowercase();
        if query.is_empty() {
            self.filtered_indices = (0..self.methods.len()).collect();
        } else {
            self.filtered_indices = self
                .methods
                .iter()
                .enumerate()
                .filter(|(_, m)| m.name.to_lowercase().contains(&query))
                .map(|(i, _)| i)
                .collect();
        }
        let len = self.filtered_indices.len();
        if len == 0 {
            self.filtered_selected = 0;
        } else if self.filtered_selected >= len {
            self.filtered_selected = len - 1;
        }
    }

    pub fn update_detail_matches(&mut self) {
        let query = self.detail_search.to_lowercase();
        self.detail_matches.clear();
        self.detail_match_index = 0;

        if let Some(result) = &self.result {
            for (i, line) in result.lines().enumerate() {
                if line.to_lowercase().contains(&query) {
                    self.detail_matches.push(i as u16);
                }
            }
        }

        if let Some(&first) = self.detail_matches.first() {
            self.result_scroll = first;
        }
    }
}

#[derive(Default)]
pub struct TransactionsTab {
    pub search_input: String,
    pub result: Option<SearchResult>,
    pub error: Option<String>,
    pub searching: bool,
    pub result_scroll: u16,
}

pub struct WalletTab {
    pub browser: MethodBrowser,
    pub wallet_name: String,
    pub wallets: Vec<String>,
    pub picker_index: usize,
    pub fetching_wallets: bool,
}

pub struct App {
    pub tab: Tab,
    pub focus: Focus,
    pub input_mode: InputMode,
    pub should_quit: bool,

    pub blockchain: Option<BlockchainInfo>,
    pub network: Option<NetworkInfo>,
    pub mempool: Option<MempoolInfo>,
    pub mining: Option<MiningInfo>,
    pub peers: Option<Vec<PeerInfo>>,
    pub recent_blocks: Vec<BlockStats>,
    pub last_tip: Option<String>,

    pub rpc_error: Option<String>,
    pub last_update: Option<Instant>,
    pub refreshing: bool,

    pub transactions: TransactionsTab,
    pub wallet: WalletTab,
    pub rpc: MethodBrowser,
}

impl Default for App {
    fn default() -> Self {
        App {
            tab: Tab::default(),
            focus: Focus::default(),
            input_mode: InputMode::default(),
            should_quit: false,
            blockchain: None,
            network: None,
            mempool: None,
            mining: None,
            peers: None,
            recent_blocks: Vec::new(),
            last_tip: None,
            rpc_error: None,
            last_update: None,
            refreshing: false,
            transactions: TransactionsTab::default(),
            wallet: WalletTab {
                browser: MethodBrowser::new(load_wallet_methods()),
                wallet_name: String::new(),
                wallets: Vec::new(),
                picker_index: 0,
                fetching_wallets: false,
            },
            rpc: MethodBrowser::new(load_non_wallet_methods()),
        }
    }
}

impl App {
    fn active_browser(&mut self) -> &mut MethodBrowser {
        match self.tab {
            Tab::Wallet => &mut self.wallet.browser,
            Tab::Rpc => &mut self.rpc,
            _ => unreachable!(),
        }
    }

    pub fn update(&mut self, event: Event) {
        match event {
            Event::Key(key) => self.handle_key(key),
            Event::Tick => {}
            Event::PollComplete(result) => self.handle_poll(*result),
            Event::SearchComplete(result) => {
                self.transactions.searching = false;
                match *result {
                    Ok(sr) => {
                        self.transactions.error = None;
                        self.transactions.result = Some(sr);
                        self.transactions.result_scroll = 0;
                    }
                    Err(e) => {
                        self.transactions.result = None;
                        self.transactions.error = Some(e);
                    }
                }
            }
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
                        self.wallet.browser.error = Some(format!("listwallets failed: {}", e));
                    }
                }
            }
            Event::WalletRpcComplete(result) => {
                self.wallet.browser.calling = false;
                match *result {
                    Ok(json) => {
                        self.wallet.browser.error = None;
                        self.wallet.browser.result = Some(json);
                        self.wallet.browser.result_scroll = 0;
                    }
                    Err(e) => {
                        self.wallet.browser.result = None;
                        self.wallet.browser.error = Some(e);
                    }
                }
            }
            Event::RpcComplete(result) => {
                self.rpc.calling = false;
                match *result {
                    Ok(json) => {
                        self.rpc.error = None;
                        self.rpc.result = Some(json);
                        self.rpc.result_scroll = 0;
                    }
                    Err(e) => {
                        self.rpc.result = None;
                        self.rpc.error = Some(e);
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

    fn handle_key(&mut self, key: KeyEvent) {
        use crossterm::event::{KeyCode, KeyModifiers};

        match self.input_mode {
            InputMode::Normal => match self.focus {
                Focus::TabBar => match key.code {
                    KeyCode::Right | KeyCode::Char('l') => self.tab = self.tab.next(),
                    KeyCode::Left | KeyCode::Char('h') => self.tab = self.tab.prev(),
                    KeyCode::Enter => self.focus = Focus::Content,
                    KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
                    _ => {}
                },
                Focus::Content => match self.tab {
                    Tab::Wallet | Tab::Rpc => self.handle_browser_content(key),
                    Tab::Transactions => self.handle_transactions_content(key),
                    _ => {
                        if key.code == KeyCode::Esc {
                            self.focus = Focus::TabBar;
                        }
                    }
                },
            },
            InputMode::TxSearch => match key.code {
                KeyCode::Esc => {
                    self.input_mode = InputMode::Normal;
                }
                KeyCode::Enter => {
                    if !self.transactions.search_input.is_empty() {
                        self.transactions.searching = true;
                        self.input_mode = InputMode::Normal;
                    }
                }
                KeyCode::Backspace => {
                    self.transactions.search_input.pop();
                }
                KeyCode::Char(c) => {
                    if !key.modifiers.contains(KeyModifiers::CONTROL) {
                        self.transactions.search_input.push(c);
                    }
                }
                _ => {}
            },
            InputMode::ArgInput => match key.code {
                KeyCode::Esc => {
                    self.input_mode = InputMode::Normal;
                    let b = self.active_browser();
                    b.editing_args = false;
                    b.arg_input.clear();
                }
                KeyCode::Enter => {
                    self.active_browser().calling = true;
                    self.active_browser().editing_args = false;
                    self.input_mode = InputMode::Normal;
                }
                KeyCode::Backspace => {
                    self.active_browser().arg_input.pop();
                }
                KeyCode::Char(c) => {
                    if !key.modifiers.contains(KeyModifiers::CONTROL) {
                        self.active_browser().arg_input.push(c);
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
            InputMode::MethodSearch => match key.code {
                KeyCode::Esc => {
                    let b = self.active_browser();
                    b.method_search.clear();
                    b.update_method_filter();
                    self.input_mode = InputMode::Normal;
                }
                KeyCode::Enter => {
                    let b = self.active_browser();
                    if !b.filtered_indices.is_empty() {
                        b.selected = b.filtered_indices[b.filtered_selected];
                        b.list_state.select(Some(b.selected));
                    }
                    b.method_search.clear();
                    b.update_method_filter();
                    self.input_mode = InputMode::Normal;
                }
                KeyCode::Backspace => {
                    let b = self.active_browser();
                    b.method_search.pop();
                    b.update_method_filter();
                }
                KeyCode::Char(c) => {
                    if !key.modifiers.contains(KeyModifiers::CONTROL) {
                        let b = self.active_browser();
                        b.method_search.push(c);
                        b.update_method_filter();
                    }
                }
                _ => {}
            },
            InputMode::DetailSearch => match key.code {
                KeyCode::Esc => {
                    let b = self.active_browser();
                    b.detail_search.clear();
                    b.detail_matches.clear();
                    self.input_mode = InputMode::Normal;
                }
                KeyCode::Enter => {
                    if !self.active_browser().detail_search.is_empty() {
                        self.active_browser().update_detail_matches();
                    }
                    self.input_mode = InputMode::Normal;
                }
                KeyCode::Backspace => {
                    self.active_browser().detail_search.pop();
                }
                KeyCode::Char(c) => {
                    if !key.modifiers.contains(KeyModifiers::CONTROL) {
                        self.active_browser().detail_search.push(c);
                    }
                }
                _ => {}
            },
        }
    }

    fn handle_transactions_content(&mut self, key: KeyEvent) {
        use crossterm::event::{KeyCode, KeyModifiers};

        match key.code {
            KeyCode::Esc => self.focus = Focus::TabBar,
            KeyCode::Char('/') => {
                self.input_mode = InputMode::TxSearch;
                self.transactions.search_input.clear();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.transactions.result_scroll = self.transactions.result_scroll.saturating_add(1);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.transactions.result_scroll = self.transactions.result_scroll.saturating_sub(1);
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.transactions.result_scroll =
                    self.transactions.result_scroll.saturating_add(20);
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.transactions.result_scroll =
                    self.transactions.result_scroll.saturating_sub(20);
            }
            _ => {}
        }
    }

    fn handle_browser_content(&mut self, key: KeyEvent) {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Esc => self.focus = Focus::TabBar,
            KeyCode::Tab => {
                let b = self.active_browser();
                b.pane = match b.pane {
                    BrowserPane::Methods => BrowserPane::Detail,
                    BrowserPane::Detail => BrowserPane::Methods,
                };
            }
            KeyCode::Char('/') => {
                let pane = self.active_browser().pane;
                match pane {
                    BrowserPane::Methods => {
                        self.input_mode = InputMode::MethodSearch;
                        let b = self.active_browser();
                        b.method_search.clear();
                        b.update_method_filter();
                    }
                    BrowserPane::Detail => {
                        self.input_mode = InputMode::DetailSearch;
                        let b = self.active_browser();
                        b.detail_search.clear();
                        b.detail_matches.clear();
                    }
                }
            }
            _ => {
                let pane = self.active_browser().pane;
                match pane {
                    BrowserPane::Methods => self.handle_methods_pane(key),
                    BrowserPane::Detail => self.handle_detail_pane(key),
                }
            }
        }
    }

    fn handle_methods_pane(&mut self, key: KeyEvent) {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                let b = self.active_browser();
                let len = b.methods.len();
                if len > 0 {
                    b.selected = (b.selected + 1) % len;
                    b.list_state.select(Some(b.selected));
                    b.result = None;
                    b.error = None;
                    b.arg_input.clear();
                    b.result_scroll = 0;
                    b.detail_search.clear();
                    b.detail_matches.clear();
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let b = self.active_browser();
                let len = b.methods.len();
                if len > 0 {
                    b.selected = (b.selected + len - 1) % len;
                    b.list_state.select(Some(b.selected));
                    b.result = None;
                    b.error = None;
                    b.arg_input.clear();
                    b.result_scroll = 0;
                    b.detail_search.clear();
                    b.detail_matches.clear();
                }
            }
            KeyCode::Char('w') if self.tab == Tab::Wallet => {
                self.wallet.fetching_wallets = true;
            }
            KeyCode::Char('G') => {
                let b = self.active_browser();
                let len = b.methods.len();
                if len > 0 {
                    b.selected = len - 1;
                    b.list_state.select(Some(b.selected));
                }
            }
            KeyCode::Char('g') => {
                let b = self.active_browser();
                if !b.methods.is_empty() {
                    b.selected = 0;
                    b.list_state.select(Some(0));
                }
            }
            _ => {}
        }
    }

    fn handle_detail_pane(&mut self, key: KeyEvent) {
        use crossterm::event::{KeyCode, KeyModifiers};

        match key.code {
            KeyCode::Enter => {
                let needs_args = {
                    let b = self.active_browser();
                    let needs = !b.methods[b.selected].params.is_empty();
                    if needs {
                        b.editing_args = true;
                    } else {
                        b.calling = true;
                    }
                    needs
                };
                if needs_args {
                    self.input_mode = InputMode::ArgInput;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.active_browser().result_scroll =
                    self.active_browser().result_scroll.saturating_add(1);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.active_browser().result_scroll =
                    self.active_browser().result_scroll.saturating_sub(1);
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.active_browser().result_scroll =
                    self.active_browser().result_scroll.saturating_add(20);
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.active_browser().result_scroll =
                    self.active_browser().result_scroll.saturating_sub(20);
            }
            KeyCode::Char('n') => {
                let b = self.active_browser();
                if !b.detail_matches.is_empty() {
                    b.detail_match_index = (b.detail_match_index + 1) % b.detail_matches.len();
                    b.result_scroll = b.detail_matches[b.detail_match_index];
                }
            }
            KeyCode::Char('N') => {
                let b = self.active_browser();
                if !b.detail_matches.is_empty() {
                    let len = b.detail_matches.len();
                    b.detail_match_index = (b.detail_match_index + len - 1) % len;
                    b.result_scroll = b.detail_matches[b.detail_match_index];
                }
            }
            _ => {}
        }
    }
}
