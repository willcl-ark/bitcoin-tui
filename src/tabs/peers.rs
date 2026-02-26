use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

use crate::app::App;
use crate::format::*;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title("Peers");

    let Some(peers) = &app.peers else {
        frame.render_widget(Paragraph::new("Loading...").block(block), area);
        return;
    };

    if peers.is_empty() {
        frame.render_widget(Paragraph::new("No peers connected").block(block), area);
        return;
    }

    let header = Row::new([
        "ID", "Address", "Type", "Net", "Dir", "Ping", "Recv", "Sent", "Height", "V2",
    ])
    .style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<Row> = peers
        .iter()
        .map(|p| {
            let dir = if p.inbound { "in" } else { "out" };
            let dir_color = if p.inbound { Color::Cyan } else { Color::Green };
            let ping = p
                .pingtime
                .map(|t| format!("{:.0}ms", t * 1000.0))
                .unwrap_or_else(|| "—".into());
            let v2 = if p.transport_protocol_type == "v2" {
                "v2"
            } else {
                "v1"
            };
            let v2_color = if v2 == "v2" {
                Color::Green
            } else {
                Color::DarkGray
            };
            let height = if p.synced_blocks >= 0 {
                fmt_number(p.synced_blocks as u64)
            } else {
                "—".into()
            };

            Row::new(vec![
                Cell::from(p.id.to_string()),
                Cell::from(p.addr.clone()),
                Cell::from(abbreviate_conn_type(&p.connection_type)),
                Cell::from(p.network.clone()),
                Cell::from(dir).style(Style::default().fg(dir_color)),
                Cell::from(ping),
                Cell::from(fmt_bytes(p.bytesrecv)),
                Cell::from(fmt_bytes(p.bytessent)),
                Cell::from(height),
                Cell::from(v2).style(Style::default().fg(v2_color)),
            ])
        })
        .collect();

    let widths = [
        ratatui::layout::Constraint::Length(5),
        ratatui::layout::Constraint::Min(20),
        ratatui::layout::Constraint::Length(7),
        ratatui::layout::Constraint::Length(5),
        ratatui::layout::Constraint::Length(3),
        ratatui::layout::Constraint::Length(8),
        ratatui::layout::Constraint::Length(9),
        ratatui::layout::Constraint::Length(9),
        ratatui::layout::Constraint::Length(8),
        ratatui::layout::Constraint::Length(3),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .column_spacing(1);

    frame.render_widget(table, area);
}

fn abbreviate_conn_type(ct: &str) -> String {
    match ct {
        "outbound-full-relay" => "full".into(),
        "block-relay-only" => "block".into(),
        "inbound" => "in".into(),
        "manual" => "manual".into(),
        "feeler" => "feeler".into(),
        "addr-fetch" => "addr".into(),
        other => other.into(),
    }
}
