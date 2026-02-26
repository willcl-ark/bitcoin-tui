use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem},
};

use crate::app::{App, Focus, InputMode};

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    super::method_browser::render(
        &app.wallet.browser,
        frame,
        area,
        app.focus == Focus::Content,
        app.input_mode,
        &app.wallet.wallet_name,
    );

    if app.input_mode == InputMode::WalletPicker {
        render_wallet_picker(app, frame, area);
    }
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
