use ratatui::{
    layout::Rect,
    widgets::{Block, Clear, Paragraph},
    Frame,
};

const HELP: &str = "\
 ↑/↓ Ctrl+P/N   move selection        PageUp/PageDown  scroll ±10
 Enter          copy command          Ctrl+Y           copy with description
 Ctrl+A         toggle offline / ai   Ctrl+U           clear query
 Tab            cycle mode            Esc              quit / cancel request
 ? (empty query) toggle this help

 Tab cycles the offline modes only; the AI engine is reached with Ctrl+A.
 AI sends your file tree, git state and container list to the configured
 provider. Commands are copied, never executed.        q / Esc closes help";

pub fn draw(frame: &mut Frame, area: Rect) {
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(HELP).block(Block::bordered().title(" help ")),
        area,
    );
}
