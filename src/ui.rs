use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs},
};

use crate::app::{App, InputMode, Tab};

pub fn render(app: &App, frame: &mut Frame) {
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(frame.area());

    render_tab_bar(app, frame, chunks[0]);
    render_content(app, frame, chunks[1]);
    render_footer(app, frame, chunks[2]);
}

fn render_tab_bar(app: &App, frame: &mut Frame, area: Rect) {
    let titles: Vec<Line> = Tab::ALL.iter().map(|t| Line::from(t.title())).collect();
    let selected = Tab::ALL.iter().position(|t| *t == app.tab).unwrap_or(0);

    let tabs = Tabs::new(titles)
        .select(selected)
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .divider("│");

    frame.render_widget(tabs, area);
}

fn render_content(app: &App, frame: &mut Frame, area: Rect) {
    match app.tab {
        Tab::Dashboard => crate::tabs::dashboard::render(app, frame, area),
        _ => {
            let block = Block::default()
                .borders(Borders::ALL)
                .title(app.tab.title());
            frame.render_widget(Paragraph::new("Loading...").block(block), area);
        }
    }
}

fn render_footer(app: &App, frame: &mut Frame, area: Rect) {
    let spans = match app.input_mode {
        InputMode::Normal => vec![
            Span::styled("Tab/←/→", Style::default().fg(Color::Yellow)),
            Span::raw(" navigate  "),
            Span::styled("/", Style::default().fg(Color::Yellow)),
            Span::raw(" search  "),
            Span::styled("q", Style::default().fg(Color::Yellow)),
            Span::raw(" quit"),
        ],
        InputMode::Search => vec![
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::raw(" search  "),
            Span::styled("Esc", Style::default().fg(Color::Yellow)),
            Span::raw(" cancel"),
        ],
    };

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}
