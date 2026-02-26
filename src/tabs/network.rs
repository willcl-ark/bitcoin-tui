use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

use crate::app::App;
use crate::format::*;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title("Network");

    let Some(info) = &app.network else {
        frame.render_widget(Paragraph::new("Loading...").block(block), area);
        return;
    };

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::vertical([
        Constraint::Length(8),
        Constraint::Length(2 + info.networks.len() as u16),
        Constraint::Min(0),
    ])
    .split(inner);

    render_info(app, frame, chunks[0]);
    render_networks(app, frame, chunks[1]);
    render_local_addresses(app, frame, chunks[2]);
}

fn render_info(app: &App, frame: &mut Frame, area: Rect) {
    let Some(info) = &app.network else { return };

    let active_color = if info.networkactive {
        Color::Green
    } else {
        Color::Red
    };

    let version_num = app
        .network
        .as_ref()
        .map(|n| n.version.to_string())
        .unwrap_or_default();

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
        kv("Version", version_num, Color::White),
        kv("Protocol", info.protocolversion.to_string(), Color::White),
        kv("Relay Fee", fmt_sat_per_vb(info.relayfee), Color::White),
        kv("Services", info.localservicesnames.join(", "), Color::White),
    ];

    frame.render_widget(Paragraph::new(lines), area);
}

fn render_networks(app: &App, frame: &mut Frame, area: Rect) {
    let Some(info) = &app.network else { return };

    if info.networks.is_empty() {
        return;
    }

    let header = Row::new(["Network", "Reachable", "Limited", "Proxy"]).style(
        Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<Row> = info
        .networks
        .iter()
        .map(|n| {
            let reachable_color = if n.reachable {
                Color::Green
            } else {
                Color::Red
            };
            Row::new(vec![
                Cell::from(n.name.clone()),
                Cell::from(if n.reachable { "yes" } else { "no" })
                    .style(Style::default().fg(reachable_color)),
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

    let table = Table::new(rows, widths).header(header).column_spacing(1);

    frame.render_widget(table, area);
}

fn render_local_addresses(app: &App, frame: &mut Frame, area: Rect) {
    let Some(info) = &app.network else { return };

    if info.localaddresses.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "No local addresses",
                Style::default().fg(Color::DarkGray),
            ))),
            area,
        );
        return;
    }

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

    let table = Table::new(rows, widths).header(header).column_spacing(1);

    frame.render_widget(table, area);
}

fn kv(key: &str, value: impl Into<String>, color: Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{:<14}", key), Style::default().fg(Color::DarkGray)),
        Span::styled(Into::<String>::into(value), Style::default().fg(color)),
    ])
}
