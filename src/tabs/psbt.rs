use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

use crate::app::{App, InputMode, PsbtFileMode};

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let chunks = Layout::horizontal([Constraint::Percentage(45), Constraint::Percentage(55)]).split(area);
    render_psbt_panel(app, frame, chunks[0]);
    render_output_panel(app, frame, chunks[1]);
    if app.psbt.picker_open {
        render_picker(app, frame, area);
    }
}

fn render_psbt_panel(app: &App, frame: &mut Frame, area: Rect) {
    let lines = if app.psbt.psbt.trim().is_empty() {
        vec![Line::from(Span::styled(
            "No PSBT loaded. Press 'l' to load from file.",
            Style::default().fg(Color::DarkGray),
        ))]
    } else {
        vec![
            Line::from(Span::styled(
                format!("Length: {} chars", app.psbt.psbt.trim().len()),
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(app.psbt.psbt.trim().to_string()),
        ]
    };

    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title("PSBT").border_style(Style::default().fg(Color::Cyan)))
            .scroll((app.psbt.scroll, 0)),
        area,
    );
}

fn render_output_panel(app: &App, frame: &mut Frame, area: Rect) {
    let mut lines: Vec<Line<'static>> = vec![
        Line::from(vec![
            Span::styled("Actions: ", Style::default().fg(Color::DarkGray)),
            Span::raw("d=decode a=analyze p=walletprocess f=finalize u=utxoupdate"),
        ]),
        Line::from(""),
    ];

    if let Some(err) = &app.psbt.error {
        lines.push(Line::from(Span::styled(
            err.clone(),
            Style::default().fg(Color::Red),
        )));
    } else if let Some(out) = &app.psbt.output {
        for line in out.lines() {
            lines.push(Line::from(line.to_string()));
        }
    } else {
        lines.push(Line::from(Span::styled(
            "No action output yet.",
            Style::default().fg(Color::DarkGray),
        )));
    }

    if let Some(action) = app.psbt.running_action {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("Running {}...", action_label(action)),
            Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
        )));
    }

    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title("Output").border_style(Style::default().fg(Color::Green)))
            .scroll((app.psbt.scroll, 0)),
        area,
    );
}

fn render_picker(app: &App, frame: &mut Frame, area: Rect) {
    let popup = Layout::vertical([Constraint::Length(area.height.saturating_sub(6))])
        .flex(Flex::Center)
        .split(area);
    let popup = Layout::horizontal([Constraint::Length(area.width.saturating_sub(8))])
        .flex(Flex::Center)
        .split(popup[0])[0];

    frame.render_widget(Clear, popup);

    let title = match app.psbt.picker_mode {
        PsbtFileMode::Load => format!("Load PSBT: {}", app.psbt.picker_dir.display()),
        PsbtFileMode::Save => format!("Save PSBT: {} (file: {})", app.psbt.picker_dir.display(), app.psbt.save_name),
    };

    let items: Vec<ListItem> = app
        .psbt
        .picker_entries
        .iter()
        .enumerate()
        .map(|(idx, entry)| {
            let prefix = if entry.is_dir { "d " } else { "f " };
            let style = if idx == app.psbt.picker_selected {
                Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)
            } else if entry.is_dir {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            };
            ListItem::new(format!("{}{}", prefix, entry.name)).style(style)
        })
        .collect();

    let mut help = vec![
        Span::styled("j/k", Style::default().fg(Color::DarkGray)),
        Span::raw(" move  "),
        Span::styled("Enter", Style::default().fg(Color::DarkGray)),
        Span::raw(" open/select  "),
        Span::styled("Esc", Style::default().fg(Color::DarkGray)),
        Span::raw(" close"),
    ];
    if app.psbt.picker_mode == PsbtFileMode::Save {
        help.push(Span::raw("  "));
        help.push(Span::styled("w", Style::default().fg(Color::DarkGray)));
        help.push(Span::raw(" write here  "));
        help.push(Span::styled("e", Style::default().fg(Color::DarkGray)));
        help.push(Span::raw(" edit filename"));
    }
    if app.input_mode == InputMode::PsbtSaveName {
        help.push(Span::raw("  "));
        help.push(Span::styled("[editing filename]", Style::default().fg(Color::Magenta)));
    }

    let chunks = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(popup);
    frame.render_widget(
        List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(Color::Cyan)),
        ),
        chunks[0],
    );
    frame.render_widget(Paragraph::new(Line::from(help)), chunks[1]);
}

fn action_label(action: crate::app::PsbtRpcAction) -> &'static str {
    match action {
        crate::app::PsbtRpcAction::Decode => "decodepsbt",
        crate::app::PsbtRpcAction::Analyze => "analyzepsbt",
        crate::app::PsbtRpcAction::WalletProcess => "walletprocesspsbt",
        crate::app::PsbtRpcAction::Finalize => "finalizepsbt",
        crate::app::PsbtRpcAction::UtxoUpdate => "utxoupdatepsbt",
    }
}
