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
    let mut step = Step::ApiKey;
    let mut draft = Draft {
        key: String::new(),
        base_url: config.ai.base_url.clone(),
        model: config.ai.model.clone(),
        selection: 0,
    };
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
            _ => &mut draft.model,
        }
    }
}

fn advance(step: &mut Step, event: KeyEvent, draft: &mut Draft) -> Flow {
    if event.code == KeyCode::Esc {
        return Flow::Exit;
    }
    if let Step::Model { models } = step {
        match event.code {
            KeyCode::Up => draft.selection = draft.selection.saturating_sub(1),
            KeyCode::Down => {
                draft.selection = (draft.selection + 1).min(models.len().saturating_sub(1))
            }
            _ => {}
        }
        if let Some(picked) = models.get(draft.selection).cloned() {
            set_model(draft, &picked);
        }
        if matches!(event.code, KeyCode::Up | KeyCode::Down) {
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
            field.target(draft).pop();
            Flow::Continue
        }
        KeyCode::Char(c) => {
            field.target(draft).push(c);
            Flow::Continue
        }
        _ => Flow::Continue,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn draft() -> Draft {
        Draft {
            key: String::new(),
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
}
