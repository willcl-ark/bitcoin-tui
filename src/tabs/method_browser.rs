use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

use crate::app::{BrowserPane, InputMode, MethodBrowser};

pub fn render(
    browser: &MethodBrowser,
    frame: &mut Frame,
    area: Rect,
    focused: bool,
    input_mode: InputMode,
    wallet_name: &str,
) {
    let cols = Layout::horizontal([Constraint::Length(30), Constraint::Min(0)]).split(area);

    render_method_list(browser, frame, cols[0], focused, input_mode, wallet_name);
    render_detail(browser, frame, cols[1], focused, input_mode, wallet_name);
}

fn pane_border_style(browser: &MethodBrowser, focused: bool, pane: BrowserPane) -> Style {
    if focused && browser.pane == pane {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    }
}

fn render_method_list(
    browser: &MethodBrowser,
    frame: &mut Frame,
    area: Rect,
    focused: bool,
    input_mode: InputMode,
    wallet_name: &str,
) {
    let is_filtered = input_mode == InputMode::MethodSearch;

    let (items, selected_in_list): (Vec<ListItem>, Option<usize>) = if is_filtered {
        let items: Vec<ListItem> = browser
            .filtered_indices
            .iter()
            .map(|&i| ListItem::new(browser.methods[i].name.as_str()))
            .collect();
        let sel = if items.is_empty() {
            None
        } else {
            Some(browser.filtered_selected)
        };
        (items, sel)
    } else {
        let items: Vec<ListItem> = browser
            .methods
            .iter()
            .map(|m| ListItem::new(m.name.as_str()))
            .collect();
        (items, Some(browser.selected))
    };

    let title = if wallet_name.is_empty() {
        "Methods".to_string()
    } else {
        format!("Methods [{}]", wallet_name)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(pane_border_style(browser, focused, BrowserPane::Methods));

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
            Span::raw(&browser.method_search),
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

        let mut state = browser.list_state;
        frame.render_stateful_widget(list, area, &mut state);
    }
}

fn render_detail(
    browser: &MethodBrowser,
    frame: &mut Frame,
    area: Rect,
    focused: bool,
    input_mode: InputMode,
    wallet_name: &str,
) {
    let is_searching = input_mode == InputMode::DetailSearch;
    let has_matches = !browser.detail_matches.is_empty();

    let (detail_area, search_area) = if is_searching || has_matches {
        let rows = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);
        (rows[0], Some(rows[1]))
    } else {
        (area, None)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Detail")
        .border_style(pane_border_style(browser, focused, BrowserPane::Detail));
    let inner = block.inner(detail_area);
    frame.render_widget(block, detail_area);

    if browser.methods.is_empty() {
        return;
    }

    let method = &browser.methods[browser.selected];
    let mut lines: Vec<Line> = Vec::new();

    if !wallet_name.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Wallet: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                wallet_name,
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
            .fg(Color::Magenta)
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

    if browser.editing_args || !browser.arg_input.is_empty() {
        lines.push(Line::from(""));
        let style = if input_mode == InputMode::ArgInput {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let cursor = if input_mode == InputMode::ArgInput {
            "_"
        } else {
            ""
        };
        lines.push(Line::from(vec![
            Span::styled("Args: ", style),
            Span::styled(&browser.arg_input, style),
            Span::styled(cursor, Style::default().fg(Color::Yellow)),
        ]));
    }

    if browser.calling {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Calling...",
            Style::default().fg(Color::Cyan),
        )));
    }

    let result_line_offset = lines.len();

    if let Some(result) = &browser.result {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Result:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )));

        let search_query = if has_matches {
            Some(browser.detail_search.to_lowercase())
        } else {
            None
        };

        for (i, rl) in result.lines().enumerate() {
            let is_match_line = browser.detail_matches.iter().any(|&m| m as usize == i);
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

    if let Some(err) = &browser.error {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("Error: {}", err),
            Style::default().fg(Color::Red),
        )));
    }

    let scroll_offset = if has_matches {
        let idx = browser
            .detail_match_index
            .min(browser.detail_matches.len().saturating_sub(1));
        let match_line = browser.detail_matches[idx];
        result_line_offset as u16 + 2 + match_line
    } else {
        browser.result_scroll
    };

    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((scroll_offset, 0));
    frame.render_widget(paragraph, inner);

    if let Some(search_area) = search_area {
        if is_searching {
            let search_line = Line::from(vec![
                Span::styled("/ ", Style::default().fg(Color::Cyan)),
                Span::raw(&browser.detail_search),
                Span::styled("_", Style::default().fg(Color::Yellow)),
            ]);
            frame.render_widget(Paragraph::new(search_line), search_area);
        } else if has_matches {
            let info = format!(
                "[{}/{}] {}",
                browser.detail_match_index + 1,
                browser.detail_matches.len(),
                browser.detail_search
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
