use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::app::App;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let zmq = &app.zmq;

    if !zmq.enabled {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("ZMQ")
            .border_style(Style::default().fg(Color::DarkGray));
        frame.render_widget(
            Paragraph::new("ZMQ not configured. Use --zmqport to enable.")
                .style(Style::default().fg(Color::DarkGray))
                .block(block),
            area,
        );
        return;
    }

    if let Some(err) = &zmq.error {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("ZMQ")
            .border_style(Style::default().fg(Color::Red));
        frame.render_widget(
            Paragraph::new(err.clone())
                .style(Style::default().fg(Color::Red))
                .block(block),
            area,
        );
        return;
    }

    if zmq.entries.is_empty() {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("ZMQ")
            .border_style(Style::default().fg(Color::DarkGray));
        frame.render_widget(
            Paragraph::new("Waiting for notifications...")
                .style(Style::default().fg(Color::DarkGray))
                .block(block),
            area,
        );
        return;
    }

    let lines: Vec<Line> = zmq
        .entries
        .iter()
        .map(|e| {
            let (label_style, hash_style) = if e.topic == "hashblock" {
                (
                    Style::default().fg(Color::Green),
                    Style::default().fg(Color::Green),
                )
            } else {
                (Style::default().fg(Color::DarkGray), Style::default())
            };
            Line::from(vec![
                Span::styled(format!("{:<12}", e.topic), label_style),
                Span::styled(&e.hash, hash_style),
            ])
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!("ZMQ ({})", zmq.entries.len()))
        .border_style(Style::default().fg(Color::Cyan));

    frame.render_widget(
        Paragraph::new(lines).block(block).scroll((zmq.scroll, 0)),
        area,
    );
}
