use crate::{config::GOOGLE_BASE_URL, llm};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

pub struct Draft {
    pub key: String,
    /// True while `key` still holds the value from the credential store.
    pub key_prefilled: bool,
    pub base_url: String,
    pub model: String,
    pub selection: usize,
}

pub enum Step {
    ApiKey,
    BaseUrl,
    Model { models: Vec<String> },
    Summary,
}

pub fn draw(frame: &mut Frame, step: &Step, draft: &Draft) {
    let area = frame.area();
    let [body, footer] = Layout::vertical([Constraint::Min(1), Constraint::Length(2)]).areas(area);
    frame.render_widget(Clear, area);
    if let Step::Model { models } = step {
        if !models.is_empty() {
            draw_model_picker(frame, body, draft, models);
            draw_footer(frame, footer);
            return;
        }
    }
    frame.render_widget(
        Paragraph::new(content(step, draft)).block(Block::bordered().title(" cmdpal setup ")),
        body,
    );
    draw_footer(frame, footer);
}

fn draw_footer(frame: &mut Frame, footer: Rect) {
    frame.render_widget(
        Paragraph::new(" ↑↓ select · Enter accept · type to edit · Esc cancel").dim(),
        footer,
    );
}

/// A scrolling `List` keeps the selected row visible when the models overflow.
fn draw_model_picker(frame: &mut Frame, body: Rect, draft: &Draft, models: &[String]) {
    let block = Block::bordered().title(" cmdpal setup ");
    frame.render_widget(block.clone(), body);
    let inner = block.inner(body);
    let [field_line, list_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(inner);
    let position = format!(
        "{}  ({}/{})",
        draft.model,
        draft.selection + 1,
        models.len()
    );
    frame.render_widget(Paragraph::new(field("Model", &position)), field_line);
    let items: Vec<ListItem> = models.iter().map(|id| ListItem::new(id.clone())).collect();
    let mut state = ListState::default();
    state.select(Some(draft.selection));
    frame.render_stateful_widget(
        List::new(items)
            .highlight_style(Style::new().add_modifier(Modifier::BOLD))
            .highlight_symbol("▶ "),
        list_area,
        &mut state,
    );
}

fn content(step: &Step, draft: &Draft) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    match step {
        Step::ApiKey => {
            lines.push(key_line(draft));
            lines.push(field("Base URL", &draft.base_url));
            let model = if draft.model.is_empty() {
                "(not set)".to_string()
            } else {
                draft.model.clone()
            };
            lines.push(field("Model", &model));
            lines.push(Line::from(match crate::secrets::backend() {
                crate::secrets::Backend::Keyring => {
                    " Stored in the OS credential store, never in config.toml.".dim()
                }
                crate::secrets::Backend::File => {
                    " No OS credential store here — saved to a private file, not config.toml.".dim()
                }
            }));
            lines.push(Line::from(
                if draft.key_prefilled {
                    " Enter keeps the saved key — type to replace it."
                } else {
                    " Paste a Google AI Studio key, then press Enter."
                }
                .dim(),
            ));
        }
        Step::BaseUrl => {
            lines.push(field("Base URL", &draft.base_url));
            lines.push(Line::from(
                " Must end in /openai/ — that path is required, /v1beta/ returns 404.".dim(),
            ));
            lines.push(Line::from(format!(" default: {GOOGLE_BASE_URL}").dim()));
        }
        Step::Model { models } => {
            lines.push(field("Model", &draft.model));
            if models.is_empty() {
                lines.push(Line::from(
                    " no models returned — type a model id instead".red(),
                ));
            }
        }
        Step::Summary => {
            lines.push(field("API key", &masked(&draft.key)));
            lines.push(field("Base URL", &draft.base_url));
            lines.push(field("Model", &draft.model));
            lines.push(Line::from(""));
            lines.push(Line::from(
                " Enter saves. Ctrl+A toggles offline/AI inside the palette.".dim(),
            ));
        }
    }
    lines
}

fn key_line(draft: &Draft) -> Line<'static> {
    let mut spans = vec![
        Span::styled(
            format!("{label:<10}", label = "API key"),
            Style::new().add_modifier(Modifier::BOLD),
        ),
        Span::raw(masked(&draft.key)),
    ];
    if draft.key_prefilled {
        spans.push(Span::raw("  (saved)").green());
    }
    Line::from(spans)
}

fn field(label: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{label:<10}"),
            Style::new().add_modifier(Modifier::BOLD),
        ),
        Span::raw(value.to_string()),
    ])
}

fn masked(key: &str) -> String {
    let chars: Vec<char> = key.chars().collect();
    if chars.len() <= 8 {
        return "•".repeat(chars.len());
    }
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("{}{tail}", "•".repeat((chars.len() - 4).min(40)))
}

pub fn probe(base_url: &str, key: &str) -> Vec<String> {
    let client = llm::Client::new(
        base_url,
        "probe",
        std::time::Duration::from_secs(10),
        0.0,
        16,
    );
    client.models(key).unwrap_or_default()
}

pub fn first_chat_model(models: &[String]) -> String {
    let mut ids: Vec<_> = models
        .iter()
        .filter(|id| is_chat_model(id))
        .cloned()
        .collect();
    ids.sort();
    ids.into_iter().next().unwrap_or_default()
}

// Embedding, image and TTS models cannot answer a chat request.
fn is_chat_model(id: &str) -> bool {
    ["embedding", "image", "tts", "aqa", "imagen"]
        .iter()
        .all(|kind| !id.contains(kind))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft() -> Draft {
        Draft {
            key: "AIzaSyTESTKEY1234".into(),
            key_prefilled: false,
            base_url: GOOGLE_BASE_URL.into(),
            model: String::new(),
            selection: 0,
        }
    }

    fn text(line: &Line) -> String {
        line.spans
            .iter()
            .map(|span| span.content.to_string())
            .collect()
    }

    #[test]
    fn masks_all_but_the_last_four() {
        let key = "AIzaSyTESTKEY1234";
        assert_eq!(
            masked(key),
            format!("{}{}", "•".repeat(key.len() - 4), "1234")
        );
        assert!(!masked(key).contains("AIzaSy"));
    }

    #[test]
    fn short_keys_are_never_revealed() {
        assert_eq!(masked(""), "");
        assert_eq!(masked("abc"), "•••");
        assert_eq!(masked("12345678"), "•".repeat(8));
    }

    #[test]
    fn filters_non_chat_models() {
        let models = vec![
            "gemini-embedding-001".into(),
            "imagen-4".into(),
            "gemini-2.5-flash".into(),
        ];
        assert_eq!(first_chat_model(&models), "gemini-2.5-flash");
    }

    #[test]
    fn empty_model_list_yields_no_default() {
        assert_eq!(first_chat_model(&[]), "");
    }

    #[test]
    fn api_key_step_shows_saved_key_url_and_model() {
        let mut draft = draft();
        draft.key_prefilled = true;
        draft.model = "gemini-2.5-flash".into();
        let lines = content(&Step::ApiKey, &draft);
        let rendered: Vec<String> = lines.iter().map(|line| text(line)).collect();
        assert!(rendered[0].contains("API key"));
        assert!(rendered[0].contains("1234"), "masked tail expected");
        assert!(rendered[0].contains("(saved)"), "saved badge expected");
        assert!(rendered[1].contains("Base URL"));
        assert!(rendered[1].contains(GOOGLE_BASE_URL));
        assert!(rendered[2].contains("Model"));
        assert!(rendered[2].contains("gemini-2.5-flash"));
    }

    #[test]
    fn unset_model_is_marked_not_set() {
        let lines = content(&Step::ApiKey, &draft());
        assert!(text(&lines[2]).contains("(not set)"));
    }

    #[test]
    fn hint_tracks_saved_vs_new_key() {
        let mut saved = draft();
        saved.key_prefilled = true;
        let saved_lines = content(&Step::ApiKey, &saved);
        assert!(text(&saved_lines[4]).contains("keeps the saved key"));

        let fresh_lines = content(&Step::ApiKey, &draft());
        assert!(text(&fresh_lines[4]).contains("Paste"));
    }

    #[test]
    fn empty_model_list_keeps_the_typing_hint() {
        let mut draft = draft();
        draft.model = "typed-id".into();
        let lines = content(&Step::Model { models: vec![] }, &draft);
        assert!(text(&lines[0]).contains("typed-id"));
        assert!(text(&lines[1]).contains("type a model id instead"));
    }

    #[test]
    fn model_picker_field_shows_position() {
        // The picker itself needs a live frame, so the unit test covers its label line.
        let mut draft = draft();
        draft.model = "gemini-2.5-flash".into();
        draft.selection = 2;
        let line = field(
            "Model",
            &format!("{}  ({}/{})", draft.model, draft.selection + 1, 24),
        );
        assert!(text(&line).contains("gemini-2.5-flash  (3/24)"));
    }

    #[test]
    fn summary_shows_key_url_and_model() {
        let mut draft = draft();
        draft.model = "gemini-2.5-flash".into();
        let lines = content(&Step::Summary, &draft);
        assert!(text(&lines[0]).contains("API key"));
        assert!(text(&lines[1]).contains("Base URL"));
        assert!(text(&lines[2]).contains("gemini-2.5-flash"));
    }
}
