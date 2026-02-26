use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Tabs},
};

use crate::app::{App, BrowserPane, Focus, InputMode, Tab};

pub fn render(app: &App, frame: &mut Frame) {
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(frame.area());

    render_tab_bar(app, frame, chunks[0]);
    render_content(app, frame, chunks[1]);
    render_footer(app, frame, chunks[2]);
}

fn render_tab_bar(app: &App, frame: &mut Frame, area: Rect) {
    let titles: Vec<Line> = Tab::ALL.iter().map(|t| Line::from(t.title())).collect();
    let selected = Tab::ALL.iter().position(|t| *t == app.tab).unwrap_or(0);

    let highlight = if app.focus == Focus::Content {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    };

    let tabs = Tabs::new(titles)
        .select(selected)
        .highlight_style(highlight)
        .divider("│");

    frame.render_widget(tabs, area);
}

fn render_content(app: &App, frame: &mut Frame, area: Rect) {
    match app.tab {
        Tab::Dashboard => crate::tabs::dashboard::render(app, frame, area),
        Tab::Mempool => crate::tabs::mempool::render(app, frame, area),
        Tab::Network => crate::tabs::network::render(app, frame, area),
        Tab::Peers => crate::tabs::peers::render(app, frame, area),
        Tab::Transactions => crate::tabs::transactions::render(app, frame, area),
        Tab::Zmq => crate::tabs::zmq::render(app, frame, area),
        Tab::Rpc => crate::tabs::rpc::render(app, frame, area),
        Tab::Wallet => crate::tabs::wallet::render(app, frame, area),
    }
}

fn render_footer(app: &App, frame: &mut Frame, area: Rect) {
    let hl = Style::default().fg(Color::Yellow);

    let left_spans = match app.input_mode {
        InputMode::Normal => match app.focus {
            Focus::TabBar => vec![
                Span::styled("D", hl),
                Span::raw("ashboard "),
                Span::styled("M", hl),
                Span::raw("empool "),
                Span::styled("N", hl),
                Span::raw("etwork "),
                Span::styled("P", hl),
                Span::raw("eers "),
                Span::styled("R", hl),
                Span::raw("PC "),
                Span::styled("W", hl),
                Span::raw("allet "),
                Span::styled("T", hl),
                Span::raw("ransactions "),
                Span::styled("Z", hl),
                Span::raw("MQ  "),
                Span::styled("q", hl),
                Span::raw(" quit"),
            ],
            Focus::Content if app.tab == Tab::Wallet || app.tab == Tab::Rpc => {
                let browser = if app.tab == Tab::Wallet {
                    &app.wallet.browser
                } else {
                    &app.rpc
                };
                match browser.pane {
                    BrowserPane::Methods => {
                        let mut spans = vec![
                            Span::styled("j/k", hl),
                            Span::raw(" methods  "),
                            Span::styled("/", hl),
                            Span::raw(" search  "),
                        ];
                        if app.tab == Tab::Wallet {
                            spans.push(Span::styled("w", hl));
                            spans.push(Span::raw(" wallet  "));
                        }
                        spans.push(Span::styled("Tab", hl));
                        spans.push(Span::raw(" pane  "));
                        spans.push(Span::styled("Esc", hl));
                        spans.push(Span::raw(" back"));
                        spans
                    }
                    BrowserPane::Detail => {
                        let mut spans = vec![
                            Span::styled("Enter", hl),
                            Span::raw(" call  "),
                            Span::styled("j/k", hl),
                            Span::raw(" scroll  "),
                            Span::styled("C-u/d", hl),
                            Span::raw(" page  "),
                            Span::styled("/", hl),
                            Span::raw(" search  "),
                        ];
                        if !browser.detail_matches.is_empty() {
                            spans.push(Span::styled("n/N", hl));
                            spans.push(Span::raw(" next/prev  "));
                        }
                        spans.push(Span::styled("Tab", hl));
                        spans.push(Span::raw(" pane  "));
                        spans.push(Span::styled("Esc", hl));
                        spans.push(Span::raw(" back"));
                        spans
                    }
                }
            }
            Focus::Content if app.tab == Tab::Zmq => vec![
                Span::styled("j/k", hl),
                Span::raw(" scroll  "),
                Span::styled("C-u/d", hl),
                Span::raw(" page  "),
                Span::styled("G", hl),
                Span::raw(" bottom  "),
                Span::styled("Esc", hl),
                Span::raw(" back"),
            ],
            Focus::Content if app.tab == Tab::Transactions => vec![
                Span::styled("/", hl),
                Span::raw(" search  "),
                Span::styled("j/k", hl),
                Span::raw(" scroll  "),
                Span::styled("C-u/d", hl),
                Span::raw(" page  "),
                Span::styled("Esc", hl),
                Span::raw(" back"),
            ],
            Focus::Content => vec![Span::styled("Esc", hl), Span::raw(" back")],
        },
        InputMode::TxSearch => vec![
            Span::styled("Enter", hl),
            Span::raw(" search  "),
            Span::styled("Esc", hl),
            Span::raw(" cancel"),
        ],
        InputMode::ArgInput => vec![
            Span::styled("Enter", hl),
            Span::raw(" send  "),
            Span::styled("Esc", hl),
            Span::raw(" cancel"),
        ],
        InputMode::WalletPicker => vec![
            Span::styled("j/k", hl),
            Span::raw(" select  "),
            Span::styled("Enter", hl),
            Span::raw(" confirm  "),
            Span::styled("Esc", hl),
            Span::raw(" cancel"),
        ],
        InputMode::MethodSearch => vec![
            Span::styled("Enter", hl),
            Span::raw(" accept  "),
            Span::styled("Esc", hl),
            Span::raw(" cancel"),
        ],
        InputMode::DetailSearch => vec![
            Span::styled("Enter", hl),
            Span::raw(" search  "),
            Span::styled("Esc", hl),
            Span::raw(" cancel"),
        ],
    };

    let right_text = if let Some(err) = &app.rpc_error {
        Span::styled(err.clone(), Style::default().fg(Color::Red))
    } else if let Some(t) = app.last_update {
        Span::styled(
            format!("↻ {}s ago", t.elapsed().as_secs()),
            Style::default().fg(Color::DarkGray),
        )
    } else {
        Span::raw("")
    };

    let cols = Layout::horizontal([Constraint::Min(0), Constraint::Length(20)]).split(area);

    frame.render_widget(Paragraph::new(Line::from(left_spans)), cols[0]);
    frame.render_widget(
        Paragraph::new(Line::from(right_text)).alignment(ratatui::layout::Alignment::Right),
        cols[1],
    );
}
