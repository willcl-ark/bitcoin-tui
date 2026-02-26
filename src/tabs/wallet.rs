use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

use crate::app::{App, Focus, InputMode, WalletPane};

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let cols = Layout::horizontal([Constraint::Length(30), Constraint::Min(0)]).split(area);

    render_method_list(app, frame, cols[0]);
    render_detail(app, frame, cols[1]);

    if app.input_mode == InputMode::WalletPicker {
        render_wallet_picker(app, frame, area);
    }
}

fn pane_border_style(app: &App, pane: WalletPane) -> Style {
    if app.focus == Focus::Content && app.wallet.pane == pane {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    }
}

fn render_method_list(app: &App, frame: &mut Frame, area: Rect) {
    let is_filtered = app.input_mode == InputMode::MethodSearch;

    let (items, selected_in_list): (Vec<ListItem>, Option<usize>) = if is_filtered {
        let items: Vec<ListItem> = app
            .wallet
            .filtered_indices
            .iter()
            .map(|&i| ListItem::new(app.wallet.methods[i].name.as_str()))
            .collect();
        let sel = if items.is_empty() {
            None
        } else {
            Some(app.wallet.filtered_selected)
        };
        (items, sel)
    } else {
        let items: Vec<ListItem> = app
            .wallet
            .methods
            .iter()
            .map(|m| ListItem::new(m.name.as_str()))
            .collect();
        (items, Some(app.wallet.selected))
    };

    let title = if app.wallet.wallet_name.is_empty() {
        "Methods".to_string()
    } else {
        format!("Methods [{}]", app.wallet.wallet_name)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(pane_border_style(app, WalletPane::Methods));

    if is_filtered {
        let rows = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);

        let list = List::new(items)
            .block(block)
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        let mut state = ratatui::widgets::ListState::default();
        state.select(selected_in_list);
        frame.render_stateful_widget(list, rows[0], &mut state);

        let search_line = Line::from(vec![
            Span::styled("/ ", Style::default().fg(Color::Cyan)),
            Span::raw(&app.wallet.method_search),
            Span::styled("_", Style::default().fg(Color::Yellow)),
        ]);
        frame.render_widget(Paragraph::new(search_line), rows[1]);
    } else {
        let list = List::new(items)
            .block(block)
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        let mut state = app.wallet.list_state;
        frame.render_stateful_widget(list, area, &mut state);
    }
}

fn render_detail(app: &App, frame: &mut Frame, area: Rect) {
    let is_searching = app.input_mode == InputMode::DetailSearch;
    let has_matches = !app.wallet.detail_matches.is_empty();

    let (detail_area, search_area) = if is_searching || has_matches {
        let rows = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);
        (rows[0], Some(rows[1]))
    } else {
        (area, None)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Detail")
        .border_style(pane_border_style(app, WalletPane::Detail));
    let inner = block.inner(detail_area);
    frame.render_widget(block, detail_area);

    if app.wallet.methods.is_empty() {
        return;
    }

    let method = &app.wallet.methods[app.wallet.selected];
    let mut lines: Vec<Line> = Vec::new();

    if !app.wallet.wallet_name.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Wallet: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                &app.wallet.wallet_name,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(""));
    }

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

    if app.wallet.editing_args || !app.wallet.arg_input.is_empty() {
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

    let result_line_offset = lines.len();

    if let Some(result) = &app.wallet.result {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Result:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )));

        let search_query = if has_matches {
            Some(app.wallet.detail_search.to_lowercase())
        } else {
            None
        };

        for (i, rl) in result.lines().enumerate() {
            let is_match_line = app.wallet.detail_matches.iter().any(|&m| m as usize == i);
            if is_match_line {
                if let Some(ref query) = search_query {
                    lines.push(highlight_line(rl, query));
                } else {
                    lines.push(Line::from(rl.to_string()));
                }
            } else {
                lines.push(Line::from(rl.to_string()));
            }
        }
    }

    if let Some(err) = &app.wallet.error {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("Error: {}", err),
            Style::default().fg(Color::Red),
        )));
    }

    let scroll_offset = if has_matches {
        let idx = app
            .wallet
            .detail_match_index
            .min(app.wallet.detail_matches.len().saturating_sub(1));
        let match_line = app.wallet.detail_matches[idx];
        result_line_offset as u16 + 2 + match_line
    } else {
        app.wallet.result_scroll
    };

    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((scroll_offset, 0));
    frame.render_widget(paragraph, inner);

    if let Some(search_area) = search_area {
        if is_searching {
            let search_line = Line::from(vec![
                Span::styled("/ ", Style::default().fg(Color::Cyan)),
                Span::raw(&app.wallet.detail_search),
                Span::styled("_", Style::default().fg(Color::Yellow)),
            ]);
            frame.render_widget(Paragraph::new(search_line), search_area);
        } else if has_matches {
            let info = format!(
                "[{}/{}] {}",
                app.wallet.detail_match_index + 1,
                app.wallet.detail_matches.len(),
                app.wallet.detail_search
            );
            let search_line = Line::from(Span::styled(info, Style::default().fg(Color::Cyan)));
            frame.render_widget(Paragraph::new(search_line), search_area);
        }
    }
}

fn highlight_line<'a>(line: &str, query: &str) -> Line<'a> {
    let lower = line.to_lowercase();
    let mut spans = Vec::new();
    let mut pos = 0;

    while let Some(start) = lower[pos..].find(query) {
        let start = pos + start;
        if start > pos {
            spans.push(Span::raw(line[pos..start].to_string()));
        }
        spans.push(Span::styled(
            line[start..start + query.len()].to_string(),
            Style::default()
                .bg(Color::Yellow)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        ));
        pos = start + query.len();
    }

    if pos < line.len() {
        spans.push(Span::raw(line[pos..].to_string()));
    }

    Line::from(spans)
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
