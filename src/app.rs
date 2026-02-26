use crossterm::event::KeyEvent;

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

pub enum Event {
    Key(KeyEvent),
    Tick,
}

#[derive(Default)]
pub struct App {
    pub tab: Tab,
    pub input_mode: InputMode,
    pub search_input: String,
    pub should_quit: bool,
}

impl App {
    pub fn update(&mut self, event: Event) {
        match event {
            Event::Key(key) => self.handle_key(key),
            Event::Tick => {}
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        use crossterm::event::{KeyCode, KeyModifiers};

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
