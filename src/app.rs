use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::Instant;

use crossterm::event::KeyEvent;
use ratatui::widgets::ListState;

use crate::peers_query::{self, PeerQuery};
use crate::rpc_types::*;
use crate::wallet_schema::{RpcMethod, load_non_wallet_methods, load_wallet_methods};

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Dashboard,
    Peers,
    Psbt,
    Transactions,
    Zmq,
    Rpc,
    Wallet,
}

impl Tab {
    pub const ALL: [Tab; 7] = [
        Tab::Dashboard,
        Tab::Peers,
        Tab::Psbt,
        Tab::Rpc,
        Tab::Wallet,
        Tab::Transactions,
        Tab::Zmq,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Tab::Dashboard => "Dashboard",
            Tab::Peers => "Peers",
            Tab::Psbt => "PSBT",
            Tab::Rpc => "RPC",
            Tab::Wallet => "Wallet",
            Tab::Transactions => "Transactions",
            Tab::Zmq => "ZMQ",
        }
    }

    pub fn next(self) -> Tab {
        match self {
            Tab::Dashboard => Tab::Peers,
            Tab::Peers => Tab::Psbt,
            Tab::Psbt => Tab::Rpc,
            Tab::Rpc => Tab::Wallet,
            Tab::Wallet => Tab::Transactions,
            Tab::Transactions => Tab::Zmq,
            Tab::Zmq => Tab::Dashboard,
        }
    }

    pub fn prev(self) -> Tab {
        match self {
            Tab::Dashboard => Tab::Zmq,
            Tab::Peers => Tab::Dashboard,
            Tab::Psbt => Tab::Peers,
            Tab::Rpc => Tab::Psbt,
            Tab::Wallet => Tab::Rpc,
            Tab::Transactions => Tab::Wallet,
            Tab::Zmq => Tab::Transactions,
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
    PsbtSaveName,
    MethodSearch,
    DetailSearch,
    PeersQuery,
}

pub struct PollResult {
    pub blockchain: Result<BlockchainInfo, String>,
    pub network: Result<NetworkInfo, String>,
    pub mempool: Result<MempoolInfo, String>,
    pub mining: Result<MiningInfo, String>,
    pub peers: Result<Vec<PeerInfo>, String>,
    pub nettotals: Result<NetTotals, String>,
    pub chaintips: Result<Vec<ChainTip>, String>,
    pub recent_blocks: Option<Vec<BlockStats>>,
}

pub enum SearchResult {
    Mempool {
        txid: String,
        entry: MempoolEntry,
        decoded: Option<String>,
    },
    Confirmed {
        txid: String,
        tx: RawTransaction,
        decoded: Option<String>,
    },
}

pub struct ZmqEntry {
    pub topic: String,
    pub hash: String,
}

pub enum Event {
    Key(KeyEvent),
    Tick,
    PollComplete(Box<PollResult>),
    RecentBlocksComplete(Vec<BlockStats>),
    ChainTipsEnriched(Vec<ChainTip>),
    SearchComplete(u64, Box<Result<SearchResult, String>>),
    WalletRpcComplete(u64, Box<Result<String, String>>),
    RpcComplete(u64, Box<Result<String, String>>),
    WalletListComplete(Box<Result<Vec<String>, String>>),
    PsbtRpcComplete(u64, Box<Result<PsbtRpcResult, String>>),
    ZmqBlockComplete(Box<Result<String, String>>),
    ZmqMessage(Box<ZmqEntry>),
    ZmqError(String),
}

#[derive(Clone, Copy)]
pub enum PsbtRpcAction {
    Decode,
    Analyze,
    WalletProcess,
    Finalize,
    UtxoUpdate,
}

pub struct PsbtRpcResult {
    pub action: PsbtRpcAction,
    pub output_json: String,
    pub updated_psbt: Option<String>,
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
    pub request_seq: u64,
    pub in_flight_request: Option<u64>,
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
            request_seq: 0,
            in_flight_request: None,
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
    pub request_seq: u64,
    pub in_flight_request: Option<u64>,
}

#[derive(Default)]
pub struct ZmqTab {
    pub entries: VecDeque<ZmqEntry>,
    pub selected: usize,
    pub enabled: bool,
    pub error: Option<String>,
    pub block_lookup: Option<String>,
    pub block_popup: Option<String>,
    pub block_popup_error: Option<String>,
    pub block_popup_loading: bool,
    pub block_popup_scroll: u16,
    pub tx_rate: VecDeque<u64>,
    pub tx_rate_epoch: Option<Instant>,
}

pub struct WalletTab {
    pub browser: MethodBrowser,
    pub wallet_name: String,
    pub wallets: Vec<String>,
    pub picker_index: usize,
    pub fetching_wallets: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PsbtFileMode {
    Load,
    Save,
}

pub struct PsbtFileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
}

pub struct PsbtTab {
    pub psbt: String,
    pub output: Option<String>,
    pub error: Option<String>,
    pub scroll: u16,
    pub rpc_in_flight: Option<PsbtRpcAction>,
    pub running_action: Option<PsbtRpcAction>,
    pub picker_open: bool,
    pub picker_mode: PsbtFileMode,
    pub picker_dir: PathBuf,
    pub picker_entries: Vec<PsbtFileEntry>,
    pub picker_selected: usize,
    pub save_name: String,
    pub request_seq: u64,
    pub in_flight_request: Option<u64>,
}

impl Default for PsbtTab {
    fn default() -> Self {
        let picker_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        PsbtTab {
            psbt: String::new(),
            output: None,
            error: None,
            scroll: 0,
            rpc_in_flight: None,
            running_action: None,
            picker_open: false,
            picker_mode: PsbtFileMode::Load,
            picker_dir,
            picker_entries: Vec::new(),
            picker_selected: 0,
            save_name: "psbt.txt".to_string(),
            request_seq: 0,
            in_flight_request: None,
        }
    }
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
    pub nettotals: Option<NetTotals>,
    pub chaintips: Option<Vec<ChainTip>>,
    pub peers: Option<Vec<PeerInfo>>,
    pub peers_show_user_agent: bool,
    pub peers_selected: usize,
    pub peers_popup: Option<String>,
    pub peers_popup_scroll: u16,
    pub peers_query_help_open: bool,
    pub peers_query_help_scroll: u16,
    pub peers_query: PeerQuery,
    pub peers_query_input: String,
    pub peers_query_error: Option<String>,
    pub peers_query_completion_base: Option<String>,
    pub peers_query_completions: Vec<String>,
    pub peers_query_completion_index: usize,
    pub peers_visible_indices: Vec<usize>,
    pub recent_blocks: Vec<BlockStats>,
    pub last_tip: Option<String>,

    pub rpc_error: Option<String>,
    pub last_update: Option<Instant>,
    pub refreshing: bool,

    pub transactions: TransactionsTab,
    pub transactions_return_target: Option<(Tab, Focus)>,
    pub psbt: PsbtTab,
    pub zmq: ZmqTab,
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
            nettotals: None,
            chaintips: None,
            peers: None,
            peers_show_user_agent: false,
            peers_selected: 0,
            peers_popup: None,
            peers_popup_scroll: 0,
            peers_query_help_open: false,
            peers_query_help_scroll: 0,
            peers_query: PeerQuery::default(),
            peers_query_input: String::new(),
            peers_query_error: None,
            peers_query_completion_base: None,
            peers_query_completions: Vec::new(),
            peers_query_completion_index: 0,
            peers_visible_indices: Vec::new(),
            recent_blocks: Vec::new(),
            last_tip: None,
            rpc_error: None,
            last_update: None,
            refreshing: false,
            transactions: TransactionsTab::default(),
            transactions_return_target: None,
            psbt: PsbtTab::default(),
            zmq: ZmqTab::default(),
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
            Event::Tick => self.advance_tx_rate(),
            Event::PollComplete(result) => self.handle_poll(*result),
            Event::RecentBlocksComplete(blocks) => {
                self.recent_blocks = blocks;
            }
            Event::ChainTipsEnriched(tips) => {
                self.chaintips = Some(tips);
            }
            Event::SearchComplete(request_id, result) => {
                if self.transactions.in_flight_request != Some(request_id) {
                    return;
                }
                self.transactions.searching = false;
                self.transactions.in_flight_request = None;
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
            Event::PsbtRpcComplete(request_id, result) => {
                if self.psbt.in_flight_request != Some(request_id) {
                    return;
                }
                self.psbt.rpc_in_flight = None;
                self.psbt.in_flight_request = None;
                self.psbt.running_action = None;
                match *result {
                    Ok(res) => {
                        self.psbt.error = None;
                        self.psbt.output = Some(res.output_json);
                        self.psbt.scroll = 0;
                        if let Some(psbt) = res.updated_psbt {
                            self.psbt.psbt = psbt;
                        }
                        let _ = res.action;
                    }
                    Err(e) => {
                        self.psbt.error = Some(e);
                    }
                }
            }
            Event::WalletRpcComplete(request_id, result) => {
                if self.wallet.browser.in_flight_request != Some(request_id) {
                    return;
                }
                self.wallet.browser.calling = false;
                self.wallet.browser.in_flight_request = None;
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
            Event::ZmqMessage(entry) => {
                const MAX_ENTRIES: usize = 2000;
                self.zmq.error = None;
                if entry.topic == "hashtx" {
                    self.record_tx_rate();
                }
                let was_at_top = self.zmq.selected == 0;
                self.zmq.entries.push_back(*entry);
                if self.zmq.entries.len() > MAX_ENTRIES {
                    self.zmq.entries.pop_front();
                    self.zmq.selected = self.zmq.selected.saturating_sub(1);
                }
                if !was_at_top {
                    self.zmq.selected =
                        (self.zmq.selected + 1).min(self.zmq.entries.len().saturating_sub(1));
                }
            }
            Event::ZmqError(err) => {
                self.zmq.error = Some(err);
            }
            Event::ZmqBlockComplete(result) => {
                self.zmq.block_popup_loading = false;
                match *result {
                    Ok(json) => {
                        self.zmq.block_popup = Some(json);
                        self.zmq.block_popup_error = None;
                        self.zmq.block_popup_scroll = 0;
                    }
                    Err(e) => {
                        self.zmq.block_popup = None;
                        self.zmq.block_popup_error = Some(e);
                        self.zmq.block_popup_scroll = 0;
                    }
                }
            }
            Event::RpcComplete(request_id, result) => {
                if self.rpc.in_flight_request != Some(request_id) {
                    return;
                }
                self.rpc.calling = false;
                self.rpc.in_flight_request = None;
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

    const TX_RATE_BUCKET_MS: u128 = 250;
    const TX_RATE_MAX_BUCKETS: usize = 240;

    fn advance_tx_rate_to(&mut self, now: Instant) {
        let Some(epoch) = self.zmq.tx_rate_epoch else {
            return;
        };
        let elapsed_buckets =
            (now.duration_since(epoch).as_millis() / Self::TX_RATE_BUCKET_MS) as usize;
        if elapsed_buckets == 0 {
            return;
        }
        let fill = elapsed_buckets.min(Self::TX_RATE_MAX_BUCKETS);
        for _ in 0..fill {
            self.zmq.tx_rate.push_back(0);
        }
        while self.zmq.tx_rate.len() > Self::TX_RATE_MAX_BUCKETS {
            self.zmq.tx_rate.pop_front();
        }
        self.zmq.tx_rate_epoch = Some(now);
    }

    fn record_tx_rate(&mut self) {
        let now = Instant::now();
        if self.zmq.tx_rate_epoch.is_none() {
            self.zmq.tx_rate_epoch = Some(now);
            self.zmq.tx_rate.push_back(0);
        }
        self.advance_tx_rate_to(now);
        if let Some(last) = self.zmq.tx_rate.back_mut() {
            *last += 1;
        }
    }

    fn advance_tx_rate(&mut self) {
        if self.zmq.tx_rate_epoch.is_some() {
            self.advance_tx_rate_to(Instant::now());
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
        match result.nettotals {
            Ok(info) => self.nettotals = Some(info),
            Err(e) if !had_error => {
                had_error = true;
                self.rpc_error = Some(e);
            }
            _ => {}
        }
        match result.chaintips {
            Ok(tips) => self.chaintips = Some(tips),
            Err(e) if !had_error => {
                had_error = true;
                self.rpc_error = Some(e);
            }
            _ => {}
        }
        match result.peers {
            Ok(info) => {
                self.peers = Some(info);
                self.refresh_peers_view();
            }
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

    fn enter_tab(&mut self, tab: Tab) {
        self.tab = tab;
        self.focus = Focus::Content;
        self.transactions_return_target = None;
        self.input_mode = InputMode::Normal;
        if tab == Tab::Transactions {
            self.input_mode = InputMode::TxSearch;
            self.transactions.search_input.clear();
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        use crossterm::event::{KeyCode, KeyModifiers};

        match self.input_mode {
            InputMode::Normal => match self.focus {
                Focus::TabBar => match key.code {
                    KeyCode::Right | KeyCode::Char('l') => self.tab = self.tab.next(),
                    KeyCode::Left | KeyCode::Char('h') => self.tab = self.tab.prev(),
                    KeyCode::Enter => self.enter_tab(self.tab),
                    KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
                    KeyCode::Char('d') => self.tab = Tab::Dashboard,
                    KeyCode::Char('p') => self.enter_tab(Tab::Peers),
                    KeyCode::Char('b') => self.enter_tab(Tab::Psbt),
                    KeyCode::Char('r') => self.enter_tab(Tab::Rpc),
                    KeyCode::Char('w') => self.enter_tab(Tab::Wallet),
                    KeyCode::Char('t') => self.enter_tab(Tab::Transactions),
                    KeyCode::Char('z') => self.enter_tab(Tab::Zmq),
                    _ => {}
                },
                Focus::Content => match self.tab {
                    Tab::Wallet | Tab::Rpc => self.handle_browser_content(key),
                    Tab::Psbt => self.handle_psbt_content(key),
                    Tab::Transactions => self.handle_transactions_content(key),
                    Tab::Zmq => self.handle_zmq_content(key),
                    Tab::Peers => self.handle_peers_content(key),
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
            InputMode::PsbtSaveName => match key.code {
                KeyCode::Esc => self.input_mode = InputMode::Normal,
                KeyCode::Enter => self.input_mode = InputMode::Normal,
                KeyCode::Backspace => {
                    self.psbt.save_name.pop();
                }
                KeyCode::Char(c) => {
                    if !key.modifiers.contains(KeyModifiers::CONTROL) {
                        self.psbt.save_name.push(c);
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
                KeyCode::Down => {
                    let b = self.active_browser();
                    if !b.filtered_indices.is_empty() {
                        b.filtered_selected =
                            (b.filtered_selected + 1).min(b.filtered_indices.len() - 1);
                    }
                }
                KeyCode::Up => {
                    let b = self.active_browser();
                    b.filtered_selected = b.filtered_selected.saturating_sub(1);
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
            InputMode::PeersQuery => match key.code {
                KeyCode::Esc => {
                    self.input_mode = InputMode::Normal;
                    self.peers_query_input.clear();
                    self.clear_peers_query_completion();
                }
                KeyCode::Enter => {
                    let cmd = self.peers_query_input.trim().to_string();
                    if !cmd.is_empty() {
                        match peers_query::apply_command(&mut self.peers_query, &cmd) {
                            Ok(()) => {
                                self.peers_query_error = None;
                                self.refresh_peers_view();
                            }
                            Err(e) => {
                                self.peers_query_error = Some(e);
                            }
                        }
                    }
                    self.peers_query_input.clear();
                    self.clear_peers_query_completion();
                    self.input_mode = InputMode::Normal;
                }
                KeyCode::Backspace => {
                    self.peers_query_input.pop();
                    self.clear_peers_query_completion();
                }
                KeyCode::Tab => {
                    self.apply_peers_query_completion();
                }
                KeyCode::Char(c) => {
                    if !key.modifiers.contains(KeyModifiers::CONTROL) {
                        self.peers_query_input.push(c);
                        self.clear_peers_query_completion();
                    }
                }
                _ => {}
            },
        }
    }

    fn handle_transactions_content(&mut self, key: KeyEvent) {
        use crossterm::event::{KeyCode, KeyModifiers};

        match key.code {
            KeyCode::Esc => {
                if let Some((tab, focus)) = self.transactions_return_target.take() {
                    self.tab = tab;
                    self.focus = focus;
                } else {
                    self.focus = Focus::TabBar;
                }
            }
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

    fn refresh_psbt_picker(&mut self) {
        let mut entries = vec![PsbtFileEntry {
            name: "..".to_string(),
            path: self
                .psbt
                .picker_dir
                .parent()
                .map(PathBuf::from)
                .unwrap_or_else(|| self.psbt.picker_dir.clone()),
            is_dir: true,
        }];

        if let Ok(read_dir) = std::fs::read_dir(&self.psbt.picker_dir) {
            for entry in read_dir.flatten() {
                let path = entry.path();
                let is_dir = path.is_dir();
                let name = entry.file_name().to_string_lossy().to_string();
                entries.push(PsbtFileEntry { name, path, is_dir });
            }
        }
        entries.sort_by(|a, b| {
            b.is_dir
                .cmp(&a.is_dir)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });

        self.psbt.picker_entries = entries;
        if self.psbt.picker_entries.is_empty() {
            self.psbt.picker_selected = 0;
        } else {
            self.psbt.picker_selected =
                self.psbt.picker_selected.min(self.psbt.picker_entries.len() - 1);
        }
    }

    fn open_psbt_picker(&mut self, mode: PsbtFileMode) {
        self.psbt.picker_mode = mode;
        self.psbt.picker_open = true;
        self.refresh_psbt_picker();
    }

    fn load_psbt_from_file(&mut self, path: &PathBuf) {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                self.psbt.psbt = content.trim().to_string();
                self.psbt.output = None;
                self.psbt.error = None;
                self.psbt.scroll = 0;
                self.psbt.picker_open = false;
            }
            Err(e) => self.psbt.error = Some(format!("load {}: {}", path.display(), e)),
        }
    }

    fn save_psbt_to_file(&mut self, path: &PathBuf) {
        match std::fs::write(path, format!("{}\n", self.psbt.psbt.trim())) {
            Ok(_) => {
                self.psbt.error = None;
                self.psbt.output = Some(format!("saved to {}", path.display()));
                self.psbt.scroll = 0;
                self.psbt.picker_open = false;
            }
            Err(e) => self.psbt.error = Some(format!("save {}: {}", path.display(), e)),
        }
    }

    fn handle_psbt_content(&mut self, key: KeyEvent) {
        use crossterm::event::{KeyCode, KeyModifiers};

        if self.psbt.picker_open {
            match key.code {
                KeyCode::Esc => {
                    self.psbt.picker_open = false;
                    self.input_mode = InputMode::Normal;
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if !self.psbt.picker_entries.is_empty() {
                        self.psbt.picker_selected =
                            (self.psbt.picker_selected + 1).min(self.psbt.picker_entries.len() - 1);
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.psbt.picker_selected = self.psbt.picker_selected.saturating_sub(1);
                }
                KeyCode::Char('e') if self.psbt.picker_mode == PsbtFileMode::Save => {
                    self.input_mode = InputMode::PsbtSaveName;
                }
                KeyCode::Char('w') if self.psbt.picker_mode == PsbtFileMode::Save => {
                    let target = self.psbt.picker_dir.join(self.psbt.save_name.trim());
                    self.save_psbt_to_file(&target);
                }
                KeyCode::Enter => {
                    if let Some(entry) = self.psbt.picker_entries.get(self.psbt.picker_selected) {
                        if entry.is_dir {
                            self.psbt.picker_dir = entry.path.clone();
                            self.psbt.picker_selected = 0;
                            self.refresh_psbt_picker();
                        } else if self.psbt.picker_mode == PsbtFileMode::Load {
                            let path = entry.path.clone();
                            self.load_psbt_from_file(&path);
                        } else {
                            let path = entry.path.clone();
                            self.save_psbt_to_file(&path);
                        }
                    }
                }
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Esc => self.focus = Focus::TabBar,
            KeyCode::Down | KeyCode::Char('j') => self.psbt.scroll = self.psbt.scroll.saturating_add(1),
            KeyCode::Up | KeyCode::Char('k') => self.psbt.scroll = self.psbt.scroll.saturating_sub(1),
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.psbt.scroll = self.psbt.scroll.saturating_add(20);
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.psbt.scroll = self.psbt.scroll.saturating_sub(20);
            }
            KeyCode::Char('l') => self.open_psbt_picker(PsbtFileMode::Load),
            KeyCode::Char('s') => self.open_psbt_picker(PsbtFileMode::Save),
            KeyCode::Char('a')
                if !self.psbt.psbt.trim().is_empty() && self.psbt.in_flight_request.is_none() =>
            {
                self.psbt.rpc_in_flight = Some(PsbtRpcAction::Analyze);
            }
            KeyCode::Char('d')
                if !self.psbt.psbt.trim().is_empty() && self.psbt.in_flight_request.is_none() =>
            {
                self.psbt.rpc_in_flight = Some(PsbtRpcAction::Decode);
            }
            KeyCode::Char('p')
                if !self.psbt.psbt.trim().is_empty() && self.psbt.in_flight_request.is_none() =>
            {
                self.psbt.rpc_in_flight = Some(PsbtRpcAction::WalletProcess);
            }
            KeyCode::Char('f')
                if !self.psbt.psbt.trim().is_empty() && self.psbt.in_flight_request.is_none() =>
            {
                self.psbt.rpc_in_flight = Some(PsbtRpcAction::Finalize);
            }
            KeyCode::Char('u')
                if !self.psbt.psbt.trim().is_empty() && self.psbt.in_flight_request.is_none() =>
            {
                self.psbt.rpc_in_flight = Some(PsbtRpcAction::UtxoUpdate);
            }
            _ => {}
        }
    }

    fn handle_zmq_content(&mut self, key: KeyEvent) {
        use crossterm::event::{KeyCode, KeyModifiers};

        if self.zmq.block_popup_loading || self.zmq.block_popup.is_some() || self.zmq.block_popup_error.is_some() {
            match key.code {
                KeyCode::Esc => {
                    self.zmq.block_popup_loading = false;
                    self.zmq.block_popup = None;
                    self.zmq.block_popup_error = None;
                    self.zmq.block_popup_scroll = 0;
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.zmq.block_popup_scroll = self.zmq.block_popup_scroll.saturating_add(1);
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.zmq.block_popup_scroll = self.zmq.block_popup_scroll.saturating_sub(1);
                }
                KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.zmq.block_popup_scroll = self.zmq.block_popup_scroll.saturating_add(20);
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.zmq.block_popup_scroll = self.zmq.block_popup_scroll.saturating_sub(20);
                }
                _ => {}
            }
            return;
        }

        let len = self.zmq.entries.len();
        if len == 0 {
            if key.code == KeyCode::Esc {
                self.focus = Focus::TabBar;
            }
            return;
        }
        let max = len - 1;

        match key.code {
            KeyCode::Esc => self.focus = Focus::TabBar,
            KeyCode::Down | KeyCode::Char('j') => {
                self.zmq.selected = (self.zmq.selected + 1).min(max);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.zmq.selected = self.zmq.selected.saturating_sub(1);
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.zmq.selected = (self.zmq.selected + 20).min(max);
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.zmq.selected = self.zmq.selected.saturating_sub(20);
            }
            KeyCode::Char('g') => self.zmq.selected = 0,
            KeyCode::Char('G') => self.zmq.selected = max,
            KeyCode::Enter => {
                let rev_index = self.zmq.selected;
                let fwd_index = max - rev_index;
                let entry = &self.zmq.entries[fwd_index];
                if entry.topic == "hashtx" {
                    self.transactions.search_input = entry.hash.clone();
                    self.transactions.searching = true;
                    self.transactions.result = None;
                    self.transactions.error = None;
                    self.transactions.result_scroll = 0;
                    self.transactions_return_target = Some((Tab::Zmq, Focus::Content));
                    self.tab = Tab::Transactions;
                    self.focus = Focus::Content;
                    self.input_mode = InputMode::Normal;
                } else if entry.topic == "hashblock" {
                    self.zmq.block_lookup = Some(entry.hash.clone());
                    self.zmq.block_popup_loading = true;
                    self.zmq.block_popup = None;
                    self.zmq.block_popup_error = None;
                    self.zmq.block_popup_scroll = 0;
                }
            }
            _ => {}
        }
    }

    fn handle_peers_content(&mut self, key: KeyEvent) {
        use crossterm::event::{KeyCode, KeyModifiers};

        if self.peers_popup.is_some() {
            match key.code {
                KeyCode::Esc => {
                    self.peers_popup = None;
                    self.peers_popup_scroll = 0;
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.peers_popup_scroll = self.peers_popup_scroll.saturating_add(1);
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.peers_popup_scroll = self.peers_popup_scroll.saturating_sub(1);
                }
                KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.peers_popup_scroll = self.peers_popup_scroll.saturating_add(20);
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.peers_popup_scroll = self.peers_popup_scroll.saturating_sub(20);
                }
                _ => {}
            }
            return;
        }

        if self.peers_query_help_open {
            match key.code {
                KeyCode::Esc => {
                    self.peers_query_help_open = false;
                    self.peers_query_help_scroll = 0;
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.peers_query_help_scroll = self.peers_query_help_scroll.saturating_add(1);
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.peers_query_help_scroll = self.peers_query_help_scroll.saturating_sub(1);
                }
                KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.peers_query_help_scroll = self.peers_query_help_scroll.saturating_add(20);
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.peers_query_help_scroll = self.peers_query_help_scroll.saturating_sub(20);
                }
                _ => {}
            }
            return;
        }

        let len = self.peers_visible_indices.len();
        let max = len.saturating_sub(1);

        match key.code {
            KeyCode::Esc => self.focus = Focus::TabBar,
            KeyCode::Char(':') => {
                self.input_mode = InputMode::PeersQuery;
                self.peers_query_input.clear();
                self.clear_peers_query_completion();
            }
            KeyCode::Char('v') => {
                self.peers_show_user_agent = !self.peers_show_user_agent;
            }
            KeyCode::Char('?') => {
                self.peers_query_help_open = true;
                self.peers_query_help_scroll = 0;
            }
            KeyCode::Char('c') => {
                self.peers_query = PeerQuery::default();
                self.peers_query_error = None;
                self.clear_peers_query_completion();
                self.refresh_peers_view();
            }
            KeyCode::Down | KeyCode::Char('j') if len > 0 => {
                self.peers_selected = (self.peers_selected + 1).min(max);
            }
            KeyCode::Up | KeyCode::Char('k') if len > 0 => {
                self.peers_selected = self.peers_selected.saturating_sub(1);
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) && len > 0 => {
                self.peers_selected = (self.peers_selected + 20).min(max);
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) && len > 0 => {
                self.peers_selected = self.peers_selected.saturating_sub(20);
            }
            KeyCode::Enter if len > 0 => {
                self.peers_popup = self
                    .peers
                    .as_ref()
                    .and_then(|peers| {
                        let src_idx = self.peers_visible_indices.get(self.peers_selected)?;
                        peers.get(*src_idx)
                    })
                    .and_then(|peer| serde_json::to_string_pretty(peer).ok());
                self.peers_popup_scroll = 0;
            }
            _ => {}
        }
    }

    fn refresh_peers_view(&mut self) {
        let Some(peers) = &self.peers else {
            self.peers_visible_indices.clear();
            self.peers_selected = 0;
            self.peers_popup = None;
            self.peers_popup_scroll = 0;
            return;
        };

        self.peers_visible_indices = peers_query::apply(peers, &self.peers_query);
        if self.peers_visible_indices.is_empty() {
            self.peers_selected = 0;
            self.peers_popup = None;
            self.peers_popup_scroll = 0;
        } else {
            self.peers_selected = self.peers_selected.min(self.peers_visible_indices.len() - 1);
        }
    }

    fn clear_peers_query_completion(&mut self) {
        self.peers_query_completion_base = None;
        self.peers_query_completions.clear();
        self.peers_query_completion_index = 0;
    }

    fn apply_peers_query_completion(&mut self) {
        let base = self
            .peers_query_completion_base
            .clone()
            .unwrap_or_else(|| self.peers_query_input.clone());
        let same_base = self.peers_query_completion_base.as_deref() == Some(base.as_str());

        if !same_base || self.peers_query_completions.is_empty() {
            let fields = self
                .peers
                .as_deref()
                .map(peers_query::known_fields)
                .unwrap_or_default();
            self.peers_query_completions = peers_query::completion_candidates(&base, &fields);
            self.peers_query_completion_base = Some(base.clone());
            self.peers_query_completion_index = 0;
        } else {
            self.peers_query_completion_index =
                (self.peers_query_completion_index + 1) % self.peers_query_completions.len();
        }

        if let Some(next) = self
            .peers_query_completions
            .get(self.peers_query_completion_index)
        {
            self.peers_query_input = next.clone();
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
