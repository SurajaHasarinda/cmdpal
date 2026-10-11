use super::{
    render::{placeholder_spans, PLACEHOLDER_COLOR},
    state::{AiState, AppState},
};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

const SPINNER: [&str; 4] = ["⠋", "⠙", "⠹", "⠸"];

pub fn draw(frame: &mut Frame, state: &AppState, area: Rect) {
    frame.render_widget(panel(&state.ai), area);
}

fn panel(ai: &AiState) -> Paragraph<'static> {
    let block = Block::new().borders(Borders::TOP);
    match ai {
        AiState::Idle => Paragraph::new(" Ctrl+A is on — press Enter to ask the model")
            .dim()
            .block(block),
        AiState::Pending => Paragraph::new(Line::from(vec![
            SPINNER[0].fg(Color::Magenta),
            Span::raw(" asking the model… Esc cancels"),
        ]))
        .block(block),
        AiState::Ready(command) => Paragraph::new(vec![
            Line::from(placeholder_spans(
                command,
                Style::new().add_modifier(Modifier::BOLD),
            )),
            Line::from("AI-generated — not in the verified catalog. Review before running.")
                .fg(PLACEHOLDER_COLOR),
        ])
        .block(block),
        AiState::Failed(error) => {
            Paragraph::new(Line::from(format!(" AI failed: {error}").red())).block(block)
        }
    }
}
