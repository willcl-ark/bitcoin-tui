use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
};

use crate::app::App;
use crate::format::*;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let cols = Layout::horizontal([
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
    ])
    .split(area);

    render_blockchain(app, frame, cols[0]);
    render_network(app, frame, cols[1]);
    render_mempool(app, frame, cols[2]);
}

fn render_blockchain(app: &App, frame: &mut Frame, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title("Blockchain");

    let Some(info) = &app.blockchain else {
        frame.render_widget(Paragraph::new("Loading...").block(block), area);
        return;
    };

    let chain_color = match info.chain.as_str() {
        "main" => Color::Green,
        _ => Color::Yellow,
    };

    let progress = info.verificationprogress;
    let progress_color = if progress >= 0.9999 {
        Color::Green
    } else {
        Color::Yellow
    };

    let ibd_color = if info.initialblockdownload {
        Color::Red
    } else {
        Color::Green
    };

    let hashrate = app
        .mining
        .as_ref()
        .map(|m| fmt_hashrate(m.networkhashps))
        .unwrap_or_else(|| "—".into());

    let lines = vec![
        kv("Chain", info.chain.clone(), chain_color),
        kv("Blocks", fmt_number(info.blocks), Color::White),
        kv("Headers", fmt_number(info.headers), Color::White),
        kv("Difficulty", fmt_difficulty(info.difficulty), Color::White),
        kv("Hash Rate", hashrate, Color::White),
        kv(
            "IBD",
            if info.initialblockdownload {
                "yes"
            } else {
                "no"
            },
            ibd_color,
        ),
        kv(
            "Pruned",
            if info.pruned { "yes" } else { "no" },
            Color::White,
        ),
        kv("Disk", fmt_bytes(info.size_on_disk), Color::White),
    ];

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(inner);

    frame.render_widget(Paragraph::new(lines), chunks[0]);

    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(progress_color))
        .ratio(progress.min(1.0))
        .label(format!("Sync: {:.2}%", progress * 100.0));
    frame.render_widget(gauge, chunks[1]);
}

fn render_network(app: &App, frame: &mut Frame, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title("Network");

    let Some(info) = &app.network else {
        frame.render_widget(Paragraph::new("Loading...").block(block), area);
        return;
    };

    let active_color = if info.networkactive {
        Color::Green
    } else {
        Color::Red
    };

    let lines = vec![
        kv(
            "Active",
            if info.networkactive { "yes" } else { "no" },
            active_color,
        ),
        kv(
            "Connections",
            format!(
                "{} ({} in / {} out)",
                info.connections, info.connections_in, info.connections_out
            ),
            Color::White,
        ),
        kv("Version", info.subversion.clone(), Color::White),
        kv("Protocol", fmt_number(info.protocolversion), Color::White),
        kv("Relay Fee", fmt_sat_per_vb(info.relayfee), Color::White),
        kv("Services", info.localservicesnames.join(", "), Color::White),
    ];

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_mempool(app: &App, frame: &mut Frame, area: Rect) {
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
        kv("Total Fees", fmt_btc(info.total_fee.as_f64()), Color::White),
        kv(
            "Min Fee",
            fmt_sat_per_vb(info.mempoolminfee.as_f64()),
            Color::White,
        ),
    ];

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(inner);

    frame.render_widget(Paragraph::new(lines), chunks[0]);

    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(mem_color))
        .ratio(usage_ratio.min(1.0))
        .label(format!(
            "Memory: {} / {}",
            fmt_bytes(info.usage),
            fmt_bytes(info.maxmempool)
        ));
    frame.render_widget(gauge, chunks[1]);
}

fn kv(key: &str, value: impl Into<String>, color: Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{:<14}", key), Style::default().fg(Color::DarkGray)),
        Span::styled(Into::<String>::into(value), Style::default().fg(color)),
    ])
}
