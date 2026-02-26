use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::app::{App, InputMode, SearchResult};
use crate::format::*;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let tx = &app.transactions;

    let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(area);

    render_search_input(app, frame, chunks[0]);

    if tx.searching {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Result")
            .border_style(Style::default().fg(Color::DarkGray));
        frame.render_widget(
            Paragraph::new("Searching...")
                .style(Style::default().fg(Color::Cyan))
                .block(block),
            chunks[1],
        );
    } else if let Some(err) = &tx.error {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Result")
            .border_style(Style::default().fg(Color::Red));
        frame.render_widget(
            Paragraph::new(err.clone())
                .style(Style::default().fg(Color::Red))
                .block(block),
            chunks[1],
        );
    } else if let Some(result) = &tx.result {
        render_result(result, tx.result_scroll, frame, chunks[1]);
    } else {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Result")
            .border_style(Style::default().fg(Color::DarkGray));
        frame.render_widget(
            Paragraph::new("Press / to search for a transaction by txid")
                .style(Style::default().fg(Color::DarkGray))
                .block(block),
            chunks[1],
        );
    }
}

fn render_search_input(app: &App, frame: &mut Frame, area: Rect) {
    let tx = &app.transactions;
    let editing = app.input_mode == InputMode::TxSearch;

    let border_color = if editing {
        Color::Cyan
    } else {
        Color::DarkGray
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Search")
        .border_style(Style::default().fg(border_color));

    let input = if editing {
        Line::from(vec![
            Span::raw(&tx.search_input),
            Span::styled("_", Style::default().fg(Color::Yellow)),
        ])
    } else if tx.search_input.is_empty() {
        Line::from(Span::styled("txid", Style::default().fg(Color::DarkGray)))
    } else {
        Line::from(Span::raw(&tx.search_input))
    };

    frame.render_widget(
        Paragraph::new(input)
            .style(Style::default().fg(Color::White))
            .block(block),
        area,
    );
}

fn render_result(result: &SearchResult, scroll: u16, frame: &mut Frame, area: Rect) {
    let lines = match result {
        SearchResult::Mempool { txid, entry } => {
            let fee_rate = if entry.vsize > 0 {
                let fee_sats = entry.fees.base.as_f64() * 100_000_000.0;
                format!("{:.1} sat/vB", fee_sats / entry.vsize as f64)
            } else {
                "—".into()
            };
            vec![
                kv(
                    "Status",
                    "MEMPOOL",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                kv("TXID", fmt_abbreviated_hash(txid), Style::default()),
                kv("Fee", fmt_btc(entry.fees.base.as_f64()), Style::default()),
                kv("Fee Rate", &fee_rate, Style::default()),
                kv("vSize", fmt_number(entry.vsize), Style::default()),
                kv("Weight", fmt_number(entry.weight), Style::default()),
                kv(
                    "Ancestors",
                    entry.ancestorcount.to_string(),
                    Style::default(),
                ),
                kv(
                    "Descendants",
                    entry.descendantcount.to_string(),
                    Style::default(),
                ),
                kv("Age", fmt_relative_time(entry.time), Style::default()),
            ]
        }
        SearchResult::Confirmed { txid, tx } => {
            let mut lines = vec![
                kv(
                    "Status",
                    "CONFIRMED",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                kv("TXID", fmt_abbreviated_hash(txid), Style::default()),
                kv(
                    "Confs",
                    tx.confirmations
                        .map(fmt_number)
                        .unwrap_or_else(|| "—".into()),
                    Style::default(),
                ),
                kv("vSize", fmt_number(tx.vsize), Style::default()),
                kv("Weight", fmt_number(tx.weight), Style::default()),
                kv("Inputs", tx.vin.len().to_string(), Style::default()),
                kv("Outputs", tx.vout.len().to_string(), Style::default()),
            ];
            if let Some(bt) = tx.blocktime {
                lines.push(kv("Block Age", fmt_relative_time(bt), Style::default()));
            }
            lines
        }
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Result")
        .border_style(Style::default().fg(Color::Cyan));

    frame.render_widget(Paragraph::new(lines).block(block).scroll((scroll, 0)), area);
}

fn kv(key: &str, value: impl Into<String>, value_style: Style) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{:<14}", key), Style::default().fg(Color::DarkGray)),
        Span::styled(Into::<String>::into(value), value_style),
    ])
}
