use super::state::{AppState, StatusMessage};
use crate::{context, knowledge::{Command, Danger}, modes::ActiveMode};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};
use regex::Regex;
use std::sync::LazyLock;

const MIN_WIDTH: u16 = 60;
const MIN_HEIGHT: u16 = 10;
const PLACEHOLDER_COLOR: Color = Color::Indexed(208);
const HELP: &str = "\
 ↑/↓ Ctrl+P/N   move selection        PageUp/PageDown  scroll ±10
 Enter          copy command          Ctrl+Y           copy with description
 Tab            cycle mode            Ctrl+U           clear query
 Esc Ctrl+C/G   quit without copying  ?                toggle this help

 Modes: default (shell/filesystem), git, kube. Commands from other modes are
 demoted, not hidden.                                      q / Esc closes help";

static PLACEHOLDER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{[a-z_0-9]+\}").unwrap());

pub fn draw(frame: &mut Frame, state: &AppState) {
    let area = frame.area();
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        frame.render_widget(Paragraph::new("Terminal too small"), area);
        return;
    }
    let [header, input, results, preview, status] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(4),
        Constraint::Length(1),
    ])
    .areas(area);
    frame.render_widget(header_line(state), header);
    frame.render_widget(Line::from(vec!["> ".cyan().bold(), Span::raw(state.query.as_str()), "▏".dim()]), input);
    draw_results(frame, state, results);
    frame.render_widget(preview_lines(state).block(Block::new().borders(Borders::TOP)), preview);
    frame.render_widget(status_line(state), status);
    if state.show_help {
        draw_help(frame, results);
    }
}

fn header_line(state: &AppState) -> Line<'static> {
    let badge_color = match state.mode {
        ActiveMode::Windows => Color::Blue,
        ActiveMode::Linux => Color::Green,
        ActiveMode::Git => Color::Yellow,
        ActiveMode::Kube => Color::Cyan,
        ActiveMode::Docker => Color::LightBlue,
    };
    Line::from(vec![
        " cmdpal ".bold(),
        format!("[{}]", context::summary(&state.context, state.mode)).fg(badge_color),
        "  Tab mode".dim(),
    ])
}

fn draw_results(frame: &mut Frame, state: &AppState, area: Rect) {
    if state.results.is_empty() {
        let message = format!("No match for \"{}\" — press Tab to try another mode; Esc to quit", state.query);
        frame.render_widget(Paragraph::new(message).dim(), area);
        return;
    }
    let items: Vec<ListItem> = state
        .results
        .iter()
        .map(|result| ListItem::new(result_row(&result.command, result.score, area.width)))
        .collect();
    let list = List::new(items).highlight_style(Style::new().add_modifier(Modifier::REVERSED)).highlight_symbol("▶ ");
    frame.render_stateful_widget(list, area, &mut ListState::default().with_selected(Some(state.selected)));
}

fn result_row(command: &Command, score: f32, width: u16) -> Line<'static> {
    let meta = format!("[{}] [{:?}] {score:.2}", command.mode, command.danger_level).to_lowercase();
    let command_width = (width as usize).saturating_sub(meta.chars().count() + 4);
    let text: String = format!("{:<command_width$}", command.command).chars().take(command_width).collect();
    Line::from(vec![Span::styled(text, danger_style(command.danger_level)), Span::raw(" "), meta.dim()])
}

fn danger_style(danger: Danger) -> Style {
    match danger {
        Danger::High => Style::new().fg(Color::Red),
        Danger::Medium => Style::new().fg(PLACEHOLDER_COLOR),
        Danger::Low => Style::new(),
    }
}

fn preview_lines(state: &AppState) -> Paragraph<'static> {
    let Some(selected) = state.selected_result() else {
        return Paragraph::new(" type a use case…").dim();
    };
    let command = &selected.command;
    let warning = match command.danger_level {
        Danger::High => "  ⚠ high risk: may destroy data".red().bold(),
        Danger::Medium => "  ⚠ medium risk".fg(PLACEHOLDER_COLOR),
        Danger::Low => Span::raw(""),
    };
    Paragraph::new(vec![
        Line::from(placeholder_spans(&command.command, danger_style(command.danger_level))),
        Line::from(vec![Span::raw(command.description.clone()), warning]),
        Line::from(suggestions(command, state).dim()),
    ])
}

fn placeholder_spans(command: &str, base: Style) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut cursor = 0;
    for found in PLACEHOLDER.find_iter(command) {
        spans.push(Span::styled(command[cursor..found.start()].to_string(), base));
        spans.push(Span::styled(found.as_str().to_string(), Style::new().fg(PLACEHOLDER_COLOR)));
        cursor = found.end();
    }
    spans.push(Span::styled(command[cursor..].to_string(), base));
    spans
}

fn suggestions(command: &Command, state: &AppState) -> String {
    let context = &state.context;
    let git = context.git.as_ref();
    let kube = context.kube.as_ref();
    command
        .placeholders
        .iter()
        .filter_map(|placeholder| {
            let values = match placeholder.kind.as_str() {
                "path" => vec![context.cwd.display().to_string()],
                "git_branch" => git?.branches.clone(),
                "git_remote" => git?.remotes.clone(),
                "kube_namespace" => vec![kube?.namespace.clone()?],
                _ => return None,
            };
            (!values.is_empty()).then(|| format!("{} → {}", placeholder.name, values.join(", ")))
        })
        .collect::<Vec<_>>()
        .join("   ")
}

fn status_line(state: &AppState) -> Line<'static> {
    match &state.status {
        StatusMessage::Idle => Line::from(" ↑↓ select · Enter copy · Tab mode · ? help · Esc quit".dim()),
        StatusMessage::Copied(text) => Line::from(format!(" Copied: {}", text.lines().last().unwrap_or_default()).green()),
        StatusMessage::Error(error) => Line::from(format!(" Clipboard unavailable: {error}").red()),
    }
}

fn draw_help(frame: &mut Frame, area: Rect) {
    frame.render_widget(Clear, area);
    frame.render_widget(Paragraph::new(HELP).block(Block::bordered().title(" help ")), area);
}
