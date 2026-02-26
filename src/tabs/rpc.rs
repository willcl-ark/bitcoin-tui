use ratatui::Frame;
use ratatui::layout::Rect;

use crate::app::{App, Focus};

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    super::method_browser::render(
        &app.rpc,
        frame,
        area,
        app.focus == Focus::Content,
        app.input_mode,
        "",
    );
}
