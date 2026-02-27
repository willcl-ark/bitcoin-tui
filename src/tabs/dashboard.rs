use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Bar, BarChart, BarGroup, Block, Borders, Cell, Gauge, Paragraph, Row, Sparkline, Table,
    },
};

use crate::app::App;
use crate::format::*;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    if app.zmq.enabled {
        let rows = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(10),
            Constraint::Length(5),
            Constraint::Length(3),
        ])
        .split(area);

        render_kpis(app, frame, rows[0]);
        render_middle(app, frame, rows[1]);
        render_block_chart(app, frame, rows[2]);
        render_tx_rate(app, frame, rows[3]);
        render_gauges(app, frame, rows[4]);
    } else {
        let rows = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(10),
            Constraint::Length(3),
        ])
        .split(area);

        render_kpis(app, frame, rows[0]);
        render_middle(app, frame, rows[1]);
        render_block_chart(app, frame, rows[2]);
        render_gauges(app, frame, rows[3]);
    }
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
    render_kpi(frame, cols[3], "Mempool Txs", &mempool_txs, Color::White);
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
    let left =
        Layout::vertical([Constraint::Min(0), Constraint::Length(8)]).split(cols[0]);
    render_recent_blocks(app, frame, left[0]);
    render_chain_details(app, frame, left[1]);

    let right = Layout::vertical([Constraint::Min(0), Constraint::Length(8)]).split(cols[1]);
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

    let max_rows = area.height.saturating_sub(3) as usize;
    for b in app.recent_blocks.iter().rev().take(max_rows) {
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

fn render_block_chart(app: &App, frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Block Weights");

    if app.recent_blocks.is_empty() {
        frame.render_widget(Paragraph::new("Waiting for data...").block(block), area);
        return;
    }

    const MAX_WEIGHT: f64 = 4_000_000.0;
    const BAR_WIDTH: u16 = 10;
    const BAR_GAP: u16 = 2;
    let inner = block.inner(area);
    let per_bar = BAR_WIDTH + BAR_GAP;
    let bars_fit = if per_bar > 0 {
        ((inner.width + BAR_GAP) / per_bar).max(1) as usize
    } else {
        1
    };
    let start = app.recent_blocks.len().saturating_sub(bars_fit);
    let visible_blocks = &app.recent_blocks[start..];

    let bars: Vec<Bar> = visible_blocks
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
        .bar_width(BAR_WIDTH)
        .bar_gap(BAR_GAP);

    frame.render_widget(chart, area);
}

fn render_chain_details(app: &App, frame: &mut Frame, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title("Chain Details");
    let Some(info) = &app.blockchain else {
        frame.render_widget(Paragraph::new("Connecting...").block(block), area);
        return;
    };
    let lines = vec![
        kv("Best", info.bestblockhash.clone(), Color::White),
        kv("Difficulty", fmt_difficulty(info.difficulty), Color::White),
        kv("Disk", fmt_bytes(info.size_on_disk), Color::White),
        kv(
            "IBD",
            if info.initialblockdownload { "yes" } else { "no" },
            if info.initialblockdownload {
                Color::Yellow
            } else {
                Color::Green
            },
        ),
        kv(
            "Pruned",
            if info.pruned { "yes" } else { "no" },
            Color::White,
        ),
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

    let inner = block.inner(area);
    frame.render_widget(block, area);

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
        kv("User Agent", info.subversion.clone(), Color::White),
        kv("Version", info.version.to_string(), Color::White),
        kv("Protocol", fmt_number(info.protocolversion), Color::White),
        kv("Relay Fee", fmt_sat_per_vb(info.relayfee), Color::White),
        kv(
            "Services",
            info.localservicesnames.join(", "),
            Color::White,
        ),
    ];

    let net_table_h = if info.networks.is_empty() {
        0
    } else {
        1 + info.networks.len() as u16
    };
    let addr_h = if info.localaddresses.is_empty() {
        1
    } else {
        1 + info.localaddresses.len() as u16
    };

    let chunks = Layout::vertical([
        Constraint::Length(8),
        Constraint::Length(net_table_h),
        Constraint::Length(addr_h),
    ])
    .split(inner);

    frame.render_widget(Paragraph::new(lines), chunks[0]);

    if !info.networks.is_empty() {
        let header = Row::new(["Network", "Reachable", "Limited", "Proxy"]).style(
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        );
        let rows: Vec<Row> = info
            .networks
            .iter()
            .map(|n| {
                let color = if n.reachable {
                    Color::Green
                } else {
                    Color::Red
                };
                Row::new(vec![
                    Cell::from(n.name.clone()),
                    Cell::from(if n.reachable { "yes" } else { "no" })
                        .style(Style::default().fg(color)),
                    Cell::from(if n.limited { "yes" } else { "no" }),
                    Cell::from(if n.proxy.is_empty() {
                        "—".to_string()
                    } else {
                        n.proxy.clone()
                    }),
                ])
            })
            .collect();
        let widths = [
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(8),
            Constraint::Min(10),
        ];
        frame.render_widget(
            Table::new(rows, widths).header(header).column_spacing(1),
            chunks[1],
        );
    }

    if info.localaddresses.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "No local addresses",
                Style::default().fg(Color::DarkGray),
            ))),
            chunks[2],
        );
    } else {
        let header = Row::new(["Address", "Port", "Score"]).style(
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        );
        let rows: Vec<Row> = info
            .localaddresses
            .iter()
            .map(|a| {
                Row::new(vec![
                    Cell::from(a.address.clone()),
                    Cell::from(a.port.to_string()),
                    Cell::from(a.score.to_string()),
                ])
            })
            .collect();
        let widths = [
            Constraint::Min(20),
            Constraint::Length(6),
            Constraint::Length(6),
        ];
        frame.render_widget(
            Table::new(rows, widths).header(header).column_spacing(1),
            chunks[2],
        );
    }
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
        kv(
            "Memory",
            format!("{} / {}", fmt_bytes(info.usage), fmt_bytes(info.maxmempool)),
            Color::White,
        ),
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

fn render_gauges(app: &App, frame: &mut Frame, area: Rect) {
    let cols =
        Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).split(area);
    render_sync_gauge(app, frame, cols[0]);
    render_mem_gauge(app, frame, cols[1]);
}

fn render_sync_gauge(app: &App, frame: &mut Frame, area: Rect) {
    let Some(info) = &app.blockchain else {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Sync / Chain");
        frame.render_widget(Paragraph::new("Connecting...").block(block), area);
        return;
    };
    let progress = info.verificationprogress.min(1.0);
    let fill_color = if progress >= 0.9999 {
        Color::LightGreen
    } else {
        Color::Yellow
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!("Sync {:.2}%", progress * 100.0));
    let gauge = Gauge::default()
        .block(block)
        .gauge_style(Style::default().fg(fill_color).bg(Color::Black))
        .ratio(progress)
        .label("");
    frame.render_widget(gauge, area);
}

fn render_mem_gauge(app: &App, frame: &mut Frame, area: Rect) {
    let Some(info) = &app.mempool else {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Mempool Memory");
        frame.render_widget(Paragraph::new("Connecting...").block(block), area);
        return;
    };
    let usage_ratio = if info.maxmempool > 0 {
        info.usage as f64 / info.maxmempool as f64
    } else {
        0.0
    };
    let fill_color = if usage_ratio < 0.5 {
        Color::LightBlue
    } else if usage_ratio < 0.8 {
        Color::Yellow
    } else {
        Color::LightRed
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(
            "Mempool Memory {} / {}",
            fmt_bytes(info.usage),
            fmt_bytes(info.maxmempool)
        ));
    let gauge = Gauge::default()
        .block(block)
        .gauge_style(Style::default().fg(fill_color).bg(Color::Black))
        .ratio(usage_ratio.min(1.0))
        .label("");
    frame.render_widget(gauge, area);
}

fn render_tx_rate(app: &App, frame: &mut Frame, area: Rect) {
    let data: Vec<u64> = app.zmq.tx_rate.iter().copied().collect();
    let rate = data.last().copied().unwrap_or(0);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!("TX Rate  {} tx/s", rate));
    let sparkline = Sparkline::default()
        .block(block)
        .data(&data)
        .style(Style::default().fg(Color::Cyan));
    frame.render_widget(sparkline, area);
}

fn kv(key: &str, value: impl Into<String>, color: Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{:<14}", key), Style::default().fg(Color::DarkGray)),
        Span::styled(Into::<String>::into(value), Style::default().fg(color)),
    ])
}
