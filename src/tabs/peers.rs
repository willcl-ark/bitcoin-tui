use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState},
};

use crate::app::{App, InputMode};
use crate::format::*;
use crate::peers_query;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let show_query_line = app.input_mode == InputMode::PeersQuery
        || !peers_query::is_empty(&app.peers_query)
        || app.peers_query_error.is_some();
    let chunks = if show_query_line {
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area)
    } else {
        Layout::vertical([Constraint::Min(0)]).split(area)
    };
    let table_area = chunks[0];

    let block = Block::default().borders(Borders::ALL).title("Peers");

    let Some(peers) = &app.peers else {
        frame.render_widget(Paragraph::new("Loading...").block(block), table_area);
        render_query_line(app, frame, chunks.get(1).copied());
        return;
    };

    if peers.is_empty() {
        frame.render_widget(Paragraph::new("No peers connected").block(block), table_area);
        render_query_line(app, frame, chunks.get(1).copied());
        return;
    }

    if app.peers_visible_indices.is_empty() {
        frame.render_widget(
            Paragraph::new("No peers match current query").block(block),
            table_area,
        );
        render_query_line(app, frame, chunks.get(1).copied());
        return;
    }

    let peer_identity_header = if app.peers_show_user_agent {
        "User Agent"
    } else {
        "Address"
    };

    let header = Row::new([
        "ID",
        peer_identity_header,
        "Type",
        "Net",
        "Dir",
        "Ping",
        "Recv",
        "Sent",
        "Height",
        "V2",
    ])
    .style(
        Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<Row> = app
        .peers_visible_indices
        .iter()
        .filter_map(|&i| peers.get(i))
        .map(|p| {
            let dir = if p.inbound { "in" } else { "out" };
            let dir_color = if p.inbound { Color::Yellow } else { Color::Green };
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
            let peer_identity = if app.peers_show_user_agent {
                if p.subver.is_empty() {
                    "—".to_string()
                } else {
                    p.subver.clone()
                }
            } else {
                p.addr.clone()
            };

            Row::new(vec![
                Cell::from(p.id.to_string()),
                Cell::from(peer_identity),
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
        .column_spacing(1)
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut state = TableState::default();
    state.select(Some(app.peers_selected));
    frame.render_stateful_widget(table, table_area, &mut state);

    render_query_line(app, frame, chunks.get(1).copied());
    render_peer_popup(app, frame, area);
    render_query_help_popup(app, frame, area);
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

fn render_peer_popup(app: &App, frame: &mut Frame, area: Rect) {
    let Some(peer_json) = &app.peers_popup else {
        return;
    };

    let popup = Layout::vertical([Constraint::Length(area.height.saturating_sub(6))])
        .flex(Flex::Center)
        .split(area);
    let popup = Layout::horizontal([Constraint::Length(area.width.saturating_sub(8))])
        .flex(Flex::Center)
        .split(popup[0])[0];

    frame.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Peer Details (Esc to close)")
        .border_style(Style::default().fg(Color::Cyan));

    frame.render_widget(
        Paragraph::new(peer_json.clone())
            .block(block)
            .scroll((app.peers_popup_scroll, 0)),
        popup,
    );
}

fn render_query_line(app: &App, frame: &mut Frame, area: Option<Rect>) {
    let Some(area) = area else {
        return;
    };
    let text = if app.input_mode == InputMode::PeersQuery {
        format!(":{}", app.peers_query_input)
    } else if let Some(err) = &app.peers_query_error {
        format!("query error: {}", err)
    } else if !peers_query::is_empty(&app.peers_query) {
        format!("query: {}", peers_query::summary(&app.peers_query))
    } else {
        "query: none  (press : for where/sort/clear)".to_string()
    };
    frame.render_widget(Paragraph::new(text), area);
}

fn render_query_help_popup(app: &App, frame: &mut Frame, area: Rect) {
    if !app.peers_query_help_open {
        return;
    }

    let popup = Layout::vertical([Constraint::Length(area.height.saturating_sub(6))])
        .flex(Flex::Center)
        .split(area);
    let popup = Layout::horizontal([Constraint::Length(area.width.saturating_sub(8))])
        .flex(Flex::Center)
        .split(popup[0])[0];

    frame.render_widget(Clear, popup);

    let help = [
        "Peers Query Help",
        "",
        "Commands:",
        "  where <field> <op> <value> [and ...]",
        "  sort <field> [asc|desc]",
        "  clear | clear where | clear sort",
        "",
        "Operators:",
        "  ==  !=  >  >=  <  <=  ~=",
        "",
        "Notes:",
        "  - Nested fields use dot notation (e.g. bytessent_per_msg.addrv2)",
        "  - Tab completes commands/fields/operators/values",
        "  - Press Tab repeatedly to cycle completion candidates",
        "",
        "Examples:",
        "  where version == 70016 and subver ~= \"Satoshi\"",
        "  where inbound == false and network == \"ipv4\"",
        "  sort bytessent_per_msg.addrv2 desc",
        "  clear",
        "",
        "Keys:",
        "  Esc close  j/k scroll  Ctrl-u/d page",
    ]
    .join("\n");

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Peers Query Help (Esc to close)")
        .border_style(Style::default().fg(Color::Cyan));

    frame.render_widget(
        Paragraph::new(help)
            .block(block)
            .scroll((app.peers_query_help_scroll, 0)),
        popup,
    );
}
