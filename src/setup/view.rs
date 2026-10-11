use crate::{config::GOOGLE_BASE_URL, llm};
use ratatui::{
    layout::{Constraint, Layout},
    style::{Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph},
    Frame,
};

pub struct Draft {
    pub key: String,
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
    frame.render_widget(
        Paragraph::new(content(step, draft)).block(Block::bordered().title(" cmdpal setup ")),
        body,
    );
    frame.render_widget(
        Paragraph::new(" ↑↓ select · Enter accept · type to edit · Esc cancel").dim(),
        footer,
    );
}

fn content(step: &Step, draft: &Draft) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    match step {
        Step::ApiKey => {
            lines.push(field("API key", &masked(&draft.key)));
            lines.push(Line::from(
                " Stored in the OS credential store, never in config.toml.".dim(),
            ));
            lines.push(Line::from(
                " Paste a Google AI Studio key, then press Enter.".dim(),
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
            for (index, id) in models.iter().enumerate() {
                let marker = if index == draft.selection {
                    "▶ "
                } else {
                    "  "
                };
                lines.push(Line::from(format!("{marker}{id}")));
            }
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

/// Embedding, image and TTS models cannot answer a chat request.
fn is_chat_model(id: &str) -> bool {
    ["embedding", "image", "tts", "aqa", "imagen"]
        .iter()
        .all(|kind| !id.contains(kind))
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
