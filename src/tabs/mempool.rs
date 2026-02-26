use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Bar, BarChart, BarGroup, Block, Borders, Gauge, Paragraph},
};

use crate::app::App;
use crate::format::*;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let chunks = Layout::vertical([Constraint::Length(8), Constraint::Min(0)]).split(area);

    render_stats(app, frame, chunks[0]);
    render_recent_blocks(app, frame, chunks[1]);
}

fn render_stats(app: &App, frame: &mut Frame, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title("Mempool");

    let Some(info) = &app.mempool else {
        frame.render_widget(Paragraph::new("Loading...").block(block), area);
        return;
    };

    let usage_ratio = if info.maxmempool > 0 {
        info.usage as f64 / info.maxmempool as f64
    } else {
        0.0
    };
    let mem_color = if usage_ratio < 0.5 {
        Color::Cyan
    } else if usage_ratio < 0.8 {
        Color::Yellow
    } else {
        Color::Red
    };

    let lines = vec![
        kv("Transactions", fmt_number(info.size), Color::White),
        kv("Virtual Size", fmt_bytes(info.bytes), Color::White),
        kv(
            "Memory Usage",
            format!("{} / {}", fmt_bytes(info.usage), fmt_bytes(info.maxmempool)),
            Color::White,
        ),
        kv("Total Fees", fmt_btc(info.total_fee.as_f64()), Color::White),
        kv(
            "Min Fee",
            fmt_sat_per_vb(info.mempoolminfee.as_f64()),
            Color::White,
        ),
    ];

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let inner_chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(inner);

    frame.render_widget(Paragraph::new(lines), inner_chunks[0]);

    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(mem_color))
        .ratio(usage_ratio.min(1.0))
        .label(format!("{:.1}%", usage_ratio * 100.0));
    frame.render_widget(gauge, inner_chunks[1]);
}

fn render_recent_blocks(app: &App, frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Recent Blocks");

    if app.recent_blocks.is_empty() {
        frame.render_widget(Paragraph::new("Waiting for data...").block(block), area);
        return;
    }

    const MAX_WEIGHT: f64 = 4_000_000.0;

    let bars: Vec<Bar> = app
        .recent_blocks
        .iter()
        .map(|b| {
            let pct = (b.total_weight as f64 / MAX_WEIGHT * 100.0).min(100.0) as u64;
            let color = if pct >= 75 {
                Color::Green
            } else if pct >= 50 {
                Color::Yellow
            } else {
                Color::Red
            };

            Bar::default()
                .value(pct)
                .label(Line::from(fmt_number(b.height)))
                .text_value(format!("{} tx", b.txs))
                .style(Style::default().fg(color))
        })
        .collect();

    let chart = BarChart::default()
        .block(block)
        .data(BarGroup::default().bars(&bars))
        .bar_width(10)
        .bar_gap(2);

    frame.render_widget(chart, area);
}

fn kv(key: &str, value: impl Into<String>, color: Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{:<14}", key), Style::default().fg(Color::DarkGray)),
        Span::styled(Into::<String>::into(value), Style::default().fg(color)),
    ])
}
