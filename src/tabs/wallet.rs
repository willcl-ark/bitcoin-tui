use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

use crate::app::{App, InputMode, WalletFocus};

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let cols = Layout::horizontal([Constraint::Length(30), Constraint::Min(0)]).split(area);

    render_method_list(app, frame, cols[0]);
    render_detail(app, frame, cols[1]);

    if app.input_mode == InputMode::WalletPicker {
        render_wallet_picker(app, frame, area);
    }
}

fn render_method_list(app: &App, frame: &mut Frame, area: Rect) {
    let items: Vec<ListItem> = app
        .wallet
        .methods
        .iter()
        .map(|m| ListItem::new(m.name.as_str()))
        .collect();

    let title = if app.wallet.wallet_name.is_empty() {
        "Methods".to_string()
    } else {
        format!("Methods [{}]", app.wallet.wallet_name)
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    let mut state = app.wallet.list_state;
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_detail(app: &App, frame: &mut Frame, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title("Detail");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.wallet.methods.is_empty() {
        return;
    }

    let method = &app.wallet.methods[app.wallet.selected];
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(Span::styled(
        &method.name,
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    for desc_line in method.description.lines() {
        lines.push(Line::from(desc_line.to_string()));
    }

    if !method.params.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Parameters:",
            Style::default().add_modifier(Modifier::BOLD),
        )));
        for p in &method.params {
            let req = if p.required { "required" } else { "optional" };
            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", p.name), Style::default().fg(Color::Cyan)),
                Span::styled(
                    format!("({}, {})", p.schema_type, req),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
            if !p.description.is_empty() {
                for dl in p.description.lines() {
                    lines.push(Line::from(Span::styled(
                        format!("    {}", dl),
                        Style::default().fg(Color::DarkGray),
                    )));
                }
            }
        }
    }

    if app.wallet.focus == WalletFocus::Args || !app.wallet.arg_input.is_empty() {
        lines.push(Line::from(""));
        let style = if app.input_mode == InputMode::WalletArg {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let cursor = if app.input_mode == InputMode::WalletArg {
            "_"
        } else {
            ""
        };
        lines.push(Line::from(vec![
            Span::styled("Args: ", style),
            Span::styled(&app.wallet.arg_input, style),
            Span::styled(cursor, Style::default().fg(Color::Yellow)),
        ]));
    }

    if app.wallet.calling {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Calling...",
            Style::default().fg(Color::Cyan),
        )));
    }

    if let Some(result) = &app.wallet.result {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Result:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )));
        for rl in result.lines() {
            lines.push(Line::from(rl.to_string()));
        }
    }

    if let Some(err) = &app.wallet.error {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("Error: {}", err),
            Style::default().fg(Color::Red),
        )));
    }

    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((app.wallet.result_scroll, 0));
    frame.render_widget(paragraph, inner);
}

fn render_wallet_picker(app: &App, frame: &mut Frame, area: Rect) {
    let wallets = &app.wallet.wallets;
    let height = (wallets.len() as u16 + 2).min(area.height.saturating_sub(4));
    let width = wallets
        .iter()
        .map(|w| w.len() as u16)
        .max()
        .unwrap_or(10)
        .max(16)
        + 6;

    let popup = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .split(area);
    let popup = Layout::horizontal([Constraint::Length(width)])
        .flex(Flex::Center)
        .split(popup[0])[0];

    frame.render_widget(Clear, popup);

    let items: Vec<ListItem> = wallets
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let marker = if *name == app.wallet.wallet_name {
                " *"
            } else {
                ""
            };
            let style = if i == app.wallet.picker_index {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(format!("{}{}", name, marker)).style(style)
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Select Wallet")
        .border_style(Style::default().fg(Color::Cyan));

    let list = List::new(items).block(block);
    frame.render_widget(list, popup);
}
