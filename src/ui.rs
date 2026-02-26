use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Tabs},
};

use crate::app::{App, Focus, InputMode, SearchResult, Tab, WalletPane};
use crate::format::*;

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

    if let Some(result) = &app.search_result {
        render_search_overlay(result, frame, frame.area());
    } else if let Some(err) = &app.search_error {
        render_error_overlay(err, frame, frame.area());
    } else if app.searching {
        render_searching_overlay(frame, frame.area());
    }
}

fn render_tab_bar(app: &App, frame: &mut Frame, area: Rect) {
    let (tab_area, search_area) = if app.input_mode == InputMode::Search {
        let cols = Layout::horizontal([Constraint::Min(30), Constraint::Length(40)]).split(area);
        (cols[0], Some(cols[1]))
    } else {
        (area, None)
    };

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

    frame.render_widget(tabs, tab_area);

    if let Some(area) = search_area {
        let input = format!("/ {}_", app.search_input);
        frame.render_widget(
            Paragraph::new(input).style(Style::default().fg(Color::Cyan)),
            area,
        );
    }
}

fn render_content(app: &App, frame: &mut Frame, area: Rect) {
    match app.tab {
        Tab::Dashboard => crate::tabs::dashboard::render(app, frame, area),
        Tab::Mempool => crate::tabs::mempool::render(app, frame, area),
        Tab::Network => crate::tabs::network::render(app, frame, area),
        Tab::Peers => crate::tabs::peers::render(app, frame, area),
        Tab::Wallet => crate::tabs::wallet::render(app, frame, area),
    }
}

fn render_footer(app: &App, frame: &mut Frame, area: Rect) {
    let hl = Style::default().fg(Color::Yellow);

    let left_spans = if app.search_result.is_some() || app.search_error.is_some() {
        vec![
            Span::styled("Esc", hl),
            Span::raw(" dismiss  "),
            Span::styled("q", hl),
            Span::raw(" quit"),
        ]
    } else {
        match app.input_mode {
            InputMode::Normal => match app.focus {
                Focus::TabBar => vec![
                    Span::styled("←/→", hl),
                    Span::raw(" switch tab  "),
                    Span::styled("Enter", hl),
                    Span::raw(" select  "),
                    Span::styled("/", hl),
                    Span::raw(" search  "),
                    Span::styled("q", hl),
                    Span::raw(" quit"),
                ],
                Focus::Content if app.tab == Tab::Wallet => match app.wallet.pane {
                    WalletPane::Methods => vec![
                        Span::styled("j/k", hl),
                        Span::raw(" methods  "),
                        Span::styled("/", hl),
                        Span::raw(" search  "),
                        Span::styled("w", hl),
                        Span::raw(" wallet  "),
                        Span::styled("Tab", hl),
                        Span::raw(" pane  "),
                        Span::styled("Esc", hl),
                        Span::raw(" back"),
                    ],
                    WalletPane::Detail => {
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
                        if !app.wallet.detail_matches.is_empty() {
                            spans.push(Span::styled("n/N", hl));
                            spans.push(Span::raw(" next/prev  "));
                        }
                        spans.push(Span::styled("Tab", hl));
                        spans.push(Span::raw(" pane  "));
                        spans.push(Span::styled("Esc", hl));
                        spans.push(Span::raw(" back"));
                        spans
                    }
                },
                Focus::Content => vec![Span::styled("Esc", hl), Span::raw(" back")],
            },
            InputMode::Search => vec![
                Span::styled("Enter", hl),
                Span::raw(" search  "),
                Span::styled("Esc", hl),
                Span::raw(" cancel"),
            ],
            InputMode::WalletArg => vec![
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
        }
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

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .split(area);
    Layout::horizontal([Constraint::Length(width)])
        .flex(Flex::Center)
        .split(vertical[0])[0]
}

fn render_search_overlay(result: &SearchResult, frame: &mut Frame, area: Rect) {
    let lines = match result {
        SearchResult::Mempool { txid, entry } => {
            let fee_rate = if entry.vsize > 0 {
                let fee_sats = entry.fees.base.as_f64() * 100_000_000.0;
                format!("{:.1} sat/vB", fee_sats / entry.vsize as f64)
            } else {
                "—".into()
            };
            vec![
                overlay_kv(
                    "Status",
                    "MEMPOOL",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                overlay_kv("TXID", fmt_abbreviated_hash(txid), Style::default()),
                overlay_kv("Fee", fmt_btc(entry.fees.base.as_f64()), Style::default()),
                overlay_kv("Fee Rate", &fee_rate, Style::default()),
                overlay_kv("vSize", fmt_number(entry.vsize), Style::default()),
                overlay_kv("Weight", fmt_number(entry.weight), Style::default()),
                overlay_kv(
                    "Ancestors",
                    entry.ancestorcount.to_string(),
                    Style::default(),
                ),
                overlay_kv(
                    "Descendants",
                    entry.descendantcount.to_string(),
                    Style::default(),
                ),
                overlay_kv("Age", fmt_relative_time(entry.time), Style::default()),
            ]
        }
        SearchResult::Confirmed { txid, tx } => {
            let mut lines = vec![
                overlay_kv(
                    "Status",
                    "CONFIRMED",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                overlay_kv("TXID", fmt_abbreviated_hash(txid), Style::default()),
                overlay_kv(
                    "Confs",
                    tx.confirmations
                        .map(fmt_number)
                        .unwrap_or_else(|| "—".into()),
                    Style::default(),
                ),
                overlay_kv("vSize", fmt_number(tx.vsize), Style::default()),
                overlay_kv("Weight", fmt_number(tx.weight), Style::default()),
                overlay_kv("Inputs", tx.vin.len().to_string(), Style::default()),
                overlay_kv("Outputs", tx.vout.len().to_string(), Style::default()),
            ];
            if let Some(bt) = tx.blocktime {
                lines.push(overlay_kv(
                    "Block Age",
                    fmt_relative_time(bt),
                    Style::default(),
                ));
            }
            lines
        }
    };

    let height = lines.len() as u16 + 2;
    let width = 46;
    let popup = centered_rect(width, height, area);

    frame.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Transaction")
        .border_style(Style::default().fg(Color::Cyan));
    frame.render_widget(Paragraph::new(lines).block(block), popup);
}

fn render_error_overlay(err: &str, frame: &mut Frame, area: Rect) {
    let popup = centered_rect(46, 5, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Search Error")
        .border_style(Style::default().fg(Color::Red));
    frame.render_widget(
        Paragraph::new(err.to_string())
            .style(Style::default().fg(Color::Red))
            .block(block),
        popup,
    );
}

fn render_searching_overlay(frame: &mut Frame, area: Rect) {
    let popup = centered_rect(30, 3, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    frame.render_widget(
        Paragraph::new("Searching...")
            .style(Style::default().fg(Color::Cyan))
            .block(block),
        popup,
    );
}

fn overlay_kv(key: &str, value: impl Into<String>, value_style: Style) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{:<14}", key), Style::default().fg(Color::DarkGray)),
        Span::styled(Into::<String>::into(value), value_style),
    ])
}
