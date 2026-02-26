use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
};

use crate::app::App;
use crate::format::*;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let rows = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(10),
        Constraint::Length(3),
    ])
    .split(area);

    render_kpis(app, frame, rows[0]);
    render_middle(app, frame, rows[1]);
    render_bottom(app, frame, rows[2]);
}

fn render_kpis(app: &App, frame: &mut Frame, area: Rect) {
    let cols = Layout::horizontal([
        Constraint::Ratio(1, 6),
        Constraint::Ratio(1, 6),
        Constraint::Ratio(1, 6),
        Constraint::Ratio(1, 6),
        Constraint::Ratio(1, 6),
        Constraint::Ratio(1, 6),
    ])
    .split(area);

    let chain = app
        .blockchain
        .as_ref()
        .map(|b| b.chain.clone())
        .unwrap_or_else(|| "—".into());
    let chain_color = match chain.as_str() {
        "main" => Color::Green,
        "regtest" => Color::Red,
        _ => Color::Yellow,
    };

    let height = app
        .blockchain
        .as_ref()
        .map(|b| fmt_number(b.blocks))
        .unwrap_or_else(|| "—".into());
    let peers = app
        .network
        .as_ref()
        .map(|n| format!("{}", n.connections))
        .unwrap_or_else(|| "—".into());
    let mempool_txs = app
        .mempool
        .as_ref()
        .map(|m| fmt_number(m.size))
        .unwrap_or_else(|| "—".into());
    let min_fee = app
        .mempool
        .as_ref()
        .map(|m| fmt_sat_per_vb(m.mempoolminfee.as_f64()))
        .unwrap_or_else(|| "—".into());
    let hashrate = app
        .mining
        .as_ref()
        .map(|m| fmt_hashrate(m.networkhashps))
        .unwrap_or_else(|| "—".into());

    render_kpi(frame, cols[0], "Chain", &chain, chain_color);
    render_kpi(frame, cols[1], "Height", &height, Color::White);
    render_kpi(frame, cols[2], "Peers", &peers, Color::White);
    render_kpi(frame, cols[3], "Mempool", &mempool_txs, Color::White);
    render_kpi(frame, cols[4], "Min Fee", &min_fee, Color::White);
    render_kpi(frame, cols[5], "Hashrate", &hashrate, Color::White);
}

fn render_kpi(frame: &mut Frame, area: Rect, title: &str, value: &str, color: Color) {
    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            value.to_string(),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )))
        .alignment(ratatui::layout::Alignment::Center),
        inner,
    );
}

fn render_middle(app: &App, frame: &mut Frame, area: Rect) {
    let cols = Layout::horizontal([Constraint::Ratio(3, 5), Constraint::Ratio(2, 5)]).split(area);
    let left = Layout::vertical([Constraint::Ratio(3, 4), Constraint::Ratio(1, 4)]).split(cols[0]);
    render_recent_blocks(app, frame, left[0]);
    render_chain_details(app, frame, left[1]);

    let right = Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).split(cols[1]);
    render_network_compact(app, frame, right[0]);
    render_mempool_compact(app, frame, right[1]);
}

fn render_recent_blocks(app: &App, frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Recent Blocks");
    let Some(info) = &app.blockchain else {
        frame.render_widget(Paragraph::new("Connecting...").block(block), area);
        return;
    };

    let mut lines = vec![Line::from(vec![
        Span::styled(
            format!("{:<10} {:>8} {:>10} {:>8} {}", "Height", "Txs", "Size", "Fee", "Age"),
            Style::default().fg(Color::DarkGray),
        ),
    ])];

    for b in app.recent_blocks.iter().rev() {
        lines.push(Line::from(format!(
            "{:<10} {:>8} {:>10} {:>8} {}",
            b.height,
            fmt_number(b.txs),
            fmt_bytes(b.total_size),
            b.avgfeerate,
            fmt_relative_time(b.time)
        )));
    }

    if lines.len() == 1 {
        lines.push(Line::from(format!(
            "Current tip: {} ({})",
            info.blocks,
            fmt_relative_time(info.time)
        )));
    }

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_chain_details(app: &App, frame: &mut Frame, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title("Chain Details");
    let Some(info) = &app.blockchain else {
        frame.render_widget(Paragraph::new("Connecting...").block(block), area);
        return;
    };
    let lines = vec![
        kv("Best", fmt_abbreviated_hash(&info.bestblockhash), Color::White),
        kv("Difficulty", fmt_difficulty(info.difficulty), Color::White),
        kv("Disk", fmt_bytes(info.size_on_disk), Color::White),
        kv(
            "Block Time",
            if info.time > 0 {
                fmt_relative_time(info.time)
            } else {
                "—".to_string()
            },
            Color::White,
        ),
    ];
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_network_compact(app: &App, frame: &mut Frame, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title("Network");

    let Some(info) = &app.network else {
        frame.render_widget(Paragraph::new("Connecting...").block(block), area);
        return;
    };

    let active_color = if info.networkactive {
        Color::Green
    } else {
        Color::Red
    };

    let services = if info.localservicesnames.is_empty() {
        "—".to_string()
    } else if info.localservicesnames.len() <= 2 {
        info.localservicesnames.join(", ")
    } else {
        format!(
            "{}, {} +{}",
            info.localservicesnames[0],
            info.localservicesnames[1],
            info.localservicesnames.len() - 2
        )
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
        kv("Services", services, Color::White),
    ];

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_mempool_compact(app: &App, frame: &mut Frame, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title("Mempool");

    let Some(info) = &app.mempool else {
        frame.render_widget(Paragraph::new("Connecting...").block(block), area);
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
        kv("Memory", fmt_bytes(info.usage), Color::White),
        kv("Total Fees", fmt_btc(info.total_fee.as_f64()), Color::White),
        kv(
            "Min Fee",
            fmt_sat_per_vb(info.mempoolminfee.as_f64()),
            Color::White,
        ),
        kv("Unbroadcast", fmt_number(info.unbroadcastcount), Color::White),
    ];

    frame.render_widget(Paragraph::new(lines).block(block), area);
    let _ = mem_color;
}

fn render_bottom(app: &App, frame: &mut Frame, area: Rect) {
    let cols = Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).split(area);
    render_sync_gauge(app, frame, cols[0]);
    render_mem_gauge(app, frame, cols[1]);
}

fn render_sync_gauge(app: &App, frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Sync / Chain");
    let Some(info) = &app.blockchain else {
        frame.render_widget(Paragraph::new("Connecting...").block(block), area);
        return;
    };
    let progress = info.verificationprogress.min(1.0);
    let color = if progress >= 0.9999 {
        Color::Green
    } else {
        Color::Yellow
    };
    let gauge = Gauge::default()
        .block(block)
        .gauge_style(Style::default().fg(color).bg(Color::DarkGray))
        .ratio(progress)
        .label(format!(
            "{:.2}% | IBD:{} | Pruned:{}",
            progress * 100.0,
            if info.initialblockdownload { "yes" } else { "no" },
            if info.pruned { "yes" } else { "no" }
        ));
    frame.render_widget(gauge, area);
}

fn render_mem_gauge(app: &App, frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Mempool Memory");
    let Some(info) = &app.mempool else {
        frame.render_widget(Paragraph::new("Connecting...").block(block), area);
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
    let gauge = Gauge::default()
        .block(block)
        .gauge_style(Style::default().fg(mem_color).bg(Color::DarkGray))
        .ratio(usage_ratio.min(1.0))
        .label(format!("{} / {}", fmt_bytes(info.usage), fmt_bytes(info.maxmempool)));
    frame.render_widget(gauge, area);
}

fn kv(key: &str, value: impl Into<String>, color: Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{:<14}", key), Style::default().fg(Color::DarkGray)),
        Span::styled(Into::<String>::into(value), Style::default().fg(color)),
    ])
}
