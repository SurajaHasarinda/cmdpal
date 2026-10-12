mod view;

use crate::{
    config::{self, Config},
    secrets,
};
use anyhow::{bail, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::try_init;
use view::{first_chat_model, probe, Draft, Step};

enum Flow {
    Continue,
    Exit,
    Save,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    Key,
    BaseUrl,
    Model,
    Frozen,
}

pub fn run(config: &Config) -> Result<()> {
    if !secrets::available() {
        bail!("no OS credential store available — the API key cannot be stored securely here");
    }
    let mut terminal = try_init()?;
    let saved_key = secrets::load().map(|key| key.to_string());
    let mut step = Step::ApiKey;
    let mut draft = initial_draft(config, saved_key);
    let outcome = (|| -> Result<()> {
        loop {
            terminal.draw(|frame| view::draw(frame, &step, &draft))?;
            if let Event::Key(event) = event::read()? {
                if event.kind != KeyEventKind::Press {
                    continue;
                }
                match advance(&mut step, event, &mut draft) {
                    Flow::Exit => return Ok(()),
                    Flow::Continue => {}
                    Flow::Save => return persist(config, &draft),
                }
            }
        }
    })();
    ratatui::restore();
    outcome
}

// Preloads the saved key so the wizard can show and keep it.
fn initial_draft(config: &Config, saved_key: Option<String>) -> Draft {
    let key_prefilled = saved_key.is_some();
    Draft {
        key: saved_key.unwrap_or_default(),
        key_prefilled,
        base_url: config.ai.base_url.clone(),
        model: config.ai.model.clone(),
        selection: 0,
    }
}

fn persist(config: &Config, draft: &Draft) -> Result<()> {
    secrets::store(&draft.key)?;
    let mut updated = config.clone();
    updated.ai.base_url = draft.base_url.clone();
    updated.ai.model = draft.model.clone();
    updated.clamp();
    let file = config::save(&updated)?;
    println!("Saved {}", file.display());
    println!("API key stored in the OS credential store.");
    Ok(())
}

impl Field {
    fn of(step: &Step) -> Self {
        match step {
            Step::ApiKey => Field::Key,
            Step::BaseUrl => Field::BaseUrl,
            Step::Model { .. } => Field::Model,
            Step::Summary => Field::Frozen,
        }
    }

    fn target(self, draft: &mut Draft) -> &mut String {
        match self {
            Field::Key => &mut draft.key,
            Field::BaseUrl => &mut draft.base_url,
            Field::Model => &mut draft.model,
            Field::Frozen => unreachable!("the summary is not a text field"),
        }
    }
}

fn advance(step: &mut Step, event: KeyEvent, draft: &mut Draft) -> Flow {
    if event.code == KeyCode::Esc {
        return Flow::Exit;
    }
    if let Step::Model { models } = step {
        if matches!(event.code, KeyCode::Up | KeyCode::Down) {
            match event.code {
                KeyCode::Up => draft.selection = draft.selection.saturating_sub(1),
                KeyCode::Down => {
                    draft.selection = (draft.selection + 1).min(models.len().saturating_sub(1))
                }
                _ => unreachable!("guarded by the matches! above"),
            }
            if let Some(picked) = models.get(draft.selection).cloned() {
                set_model(draft, &picked);
            }
            return Flow::Continue;
        }
    }
    let field = Field::of(step);
    if field == Field::Frozen {
        return match event.code {
            KeyCode::Enter => submit(step, draft),
            _ => Flow::Continue,
        };
    }
    match event.code {
        KeyCode::Enter => submit(step, draft),
        KeyCode::Backspace => {
            discard_prefilled_key(field, draft);
            field.target(draft).pop();
            Flow::Continue
        }
        KeyCode::Char(c) => {
            discard_prefilled_key(field, draft);
            field.target(draft).push(c);
            Flow::Continue
        }
        _ => Flow::Continue,
    }
}

/// Editing a masked saved key starts fresh rather than appending invisibly.
fn discard_prefilled_key(field: Field, draft: &mut Draft) {
    if field == Field::Key && draft.key_prefilled {
        draft.key.clear();
        draft.key_prefilled = false;
    }
}

fn submit(step: &mut Step, draft: &mut Draft) -> Flow {
    match step {
        Step::ApiKey if draft.key.trim().is_empty() => Flow::Exit,
        Step::ApiKey => {
            *step = Step::BaseUrl;
            Flow::Continue
        }
        Step::BaseUrl => {
            let models = probe(&draft.base_url, &draft.key);
            if draft.model.trim().is_empty() {
                set_model(draft, &first_chat_model(&models));
            }
            draft.selection = sync_selection(&models, &draft.model);
            *step = Step::Model { models };
            Flow::Continue
        }
        Step::Model { .. } if draft.model.trim().is_empty() => Flow::Exit,
        Step::Model { .. } => {
            *step = Step::Summary;
            Flow::Continue
        }
        Step::Summary => Flow::Save,
    }
}

fn set_model(draft: &mut Draft, value: &str) {
    draft.model.clear();
    draft.model.push_str(value);
}

/// Point the highlight at the model already chosen, so marker and field agree.
fn sync_selection(models: &[String], model: &str) -> usize {
    models.iter().position(|id| id == model).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    fn draft() -> Draft {
        Draft {
            key: String::new(),
            key_prefilled: false,
            base_url: String::new(),
            model: String::new(),
            selection: 0,
        }
    }

    #[test]
    fn field_selection_follows_the_step() {
        assert_eq!(Field::of(&Step::ApiKey), Field::Key);
        assert_eq!(Field::of(&Step::BaseUrl), Field::BaseUrl);
        assert_eq!(Field::of(&Step::Summary), Field::Frozen);
    }

    #[test]
    fn blank_key_exits_without_advancing() {
        let mut step = Step::ApiKey;
        let mut draft = draft();
        assert!(matches!(submit(&mut step, &mut draft), Flow::Exit));
        assert!(matches!(step, Step::ApiKey));
    }

    #[test]
    fn a_key_advances_to_the_base_url_step() {
        let mut step = Step::ApiKey;
        let mut draft = draft();
        draft.key = "AIza".into();
        assert!(matches!(submit(&mut step, &mut draft), Flow::Continue));
        assert!(matches!(step, Step::BaseUrl));
    }

    #[test]
    fn blank_model_exits_the_model_step() {
        let mut step = Step::Model { models: vec![] };
        let mut draft = draft();
        assert!(matches!(submit(&mut step, &mut draft), Flow::Exit));
    }

    #[test]
    fn initial_draft_preloads_the_saved_key() {
        let config = Config::default();
        let draft = initial_draft(&config, Some("AIza-saved".into()));
        assert_eq!(draft.key, "AIza-saved");
        assert!(draft.key_prefilled);
        assert_eq!(draft.base_url, config.ai.base_url);
        assert_eq!(draft.model, config.ai.model);
    }

    #[test]
    fn initial_draft_starts_blank_without_a_saved_key() {
        let draft = initial_draft(&Config::default(), None);
        assert_eq!(draft.key, "");
        assert!(!draft.key_prefilled);
    }

    #[test]
    fn typing_replaces_the_saved_key() {
        let mut step = Step::ApiKey;
        let mut draft = draft();
        draft.key = "AIza-saved".into();
        draft.key_prefilled = true;
        advance(
            &mut step,
            KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
            &mut draft,
        );
        assert_eq!(draft.key, "x");
        assert!(!draft.key_prefilled);
    }

    #[test]
    fn backspace_clears_the_saved_key() {
        let mut step = Step::ApiKey;
        let mut draft = draft();
        draft.key = "AIza-saved".into();
        draft.key_prefilled = true;
        advance(
            &mut step,
            KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE),
            &mut draft,
        );
        assert_eq!(draft.key, "");
        assert!(!draft.key_prefilled);
    }

    #[test]
    fn down_arrow_moves_the_selection_and_syncs_the_field() {
        let mut step = Step::Model {
            models: vec!["a".into(), "b".into()],
        };
        let mut draft = draft();
        draft.selection = 0;
        advance(
            &mut step,
            KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
            &mut draft,
        );
        assert_eq!(draft.selection, 1);
        assert_eq!(draft.model, "b");
    }

    #[test]
    fn up_arrow_stops_at_the_top() {
        let mut step = Step::Model {
            models: vec!["a".into(), "b".into()],
        };
        let mut draft = draft();
        advance(
            &mut step,
            KeyEvent::new(KeyCode::Up, KeyModifiers::NONE),
            &mut draft,
        );
        assert_eq!(draft.selection, 0);
        assert_eq!(draft.model, "a");
    }

    #[test]
    fn sync_selection_finds_the_current_model() {
        let models = vec!["a".to_string(), "gemini-2.5-flash".to_string()];
        assert_eq!(sync_selection(&models, "gemini-2.5-flash"), 1);
        assert_eq!(sync_selection(&models, "custom-id"), 0);
        assert_eq!(sync_selection(&[], "custom-id"), 0);
    }
}
