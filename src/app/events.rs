use super::{
    render,
    state::{AppState, StatusMessage},
};
use crate::{
    clipboard::copy_to_clipboard, config::Config, knowledge::Database, llm, resolve,
    search::Searcher,
};
use anyhow::{Context as _, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::{
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
    time::Duration,
};

const PAGE_SIZE: isize = 10;
const TICK: Duration = Duration::from_millis(50);

type Answers = Receiver<Result<String, String>>;

pub fn run(
    mut state: AppState,
    searcher: &Searcher,
    database: Option<&Database>,
    config: &Config,
) -> Result<AppState> {
    let mut terminal =
        ratatui::try_init().context("cannot start the terminal UI (is stdout a terminal?)")?;
    let mut answers: Option<Answers> = None;
    let outcome = (|| -> Result<()> {
        while !state.should_quit {
            // Collect placeholder sources before drawing so the preview shows the
            // resolved command. Cached after the first hit, so this is a cheap
            // check rather than a subprocess per frame.
            state.ensure_processes();
            terminal.draw(|frame| render::draw(frame, &state))?;
            if event::poll(TICK)? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        answers = handle_key(&mut state, key, searcher, database, config, answers);
                    }
                }
            }
            if let Some(answer) = poll_answer(&answers) {
                answers = None;
                state.apply_ai(answer);
            }
        }
        Ok(())
    })();
    ratatui::restore();
    outcome.map(|()| state)
}

fn poll_answer(answers: &Option<Answers>) -> Option<Result<String, String>> {
    match answers.as_ref()?.try_recv() {
        Ok(Ok(command)) => Some(Ok(command)),
        Ok(Err(error)) => Some(Err(error.to_string())),
        Err(TryRecvError::Disconnected) => Some(Err("AI request failed".into())),
        Err(TryRecvError::Empty) => None,
    }
}

fn handle_key(
    state: &mut AppState,
    key: KeyEvent,
    searcher: &Searcher,
    database: Option<&Database>,
    config: &Config,
    answers: Option<Answers>,
) -> Option<Answers> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    if state.show_help {
        state.show_help = !matches!(key.code, KeyCode::Esc | KeyCode::Char('q' | '?'));
        return answers;
    }
    match key.code {
        KeyCode::Esc if state.is_ai_pending() => state.cancel_ai(),
        KeyCode::Esc => state.should_quit = true,
        KeyCode::Char('c' | 'g') if ctrl => state.should_quit = true,
        KeyCode::Char('a') if ctrl => {
            state.toggle_engine(searcher);
            return None;
        }
        KeyCode::Char('u') if ctrl => edit_query(state, searcher, String::clear),
        KeyCode::Char('p') if ctrl => state.move_selection(-1),
        KeyCode::Char('n') if ctrl => state.move_selection(1),
        KeyCode::Char('y') if ctrl => copy_selected(state, database, true),
        KeyCode::Up => state.move_selection(-1),
        KeyCode::Down => state.move_selection(1),
        KeyCode::PageUp => state.move_selection(-PAGE_SIZE),
        KeyCode::PageDown => state.move_selection(PAGE_SIZE),
        KeyCode::Enter if state.engine.is_ai() => {
            return if state.is_ai_pending() {
                answers
            } else {
                dispatch(state, config, answers)
            };
        }
        KeyCode::Enter => copy_selected(state, database, false),
        KeyCode::Tab => switch_mode(state, searcher),
        KeyCode::Backspace => edit_query(state, searcher, |query| {
            query.pop();
        }),
        KeyCode::Char('?') if state.query.is_empty() => state.show_help = true,
        // Windows reports AltGr characters as Ctrl+Alt, so Alt keeps them typeable.
        KeyCode::Char(character) if !ctrl || key.modifiers.contains(KeyModifiers::ALT) => {
            edit_query(state, searcher, |query| query.push(character))
        }
        _ => {}
    }
    answers
}

/// One request per Enter: typing a long query must not burn the free-tier quota.
fn dispatch(state: &mut AppState, config: &Config, answers: Option<Answers>) -> Option<Answers> {
    if state.query.trim().is_empty() {
        state.apply_ai(Err("type a request first".into()));
        return answers;
    }
    let Some(prompt) = llm::prompt_for(config, state.query.clone(), &state.context, state.mode)
    else {
        state.apply_ai(Err(llm::Error::MissingKey.to_string()));
        return answers;
    };
    state.begin_ai();
    let client = match llm::Client::from_config(config) {
        Ok(client) => client,
        Err(error) => {
            state.apply_ai(Err(error.to_string()));
            return answers;
        }
    };
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let _ = sender.send(prompt.send(&client).map_err(|error| error.to_string()));
    });
    Some(receiver)
}

fn edit_query(state: &mut AppState, searcher: &Searcher, edit: impl FnOnce(&mut String)) {
    edit(&mut state.query);
    state.status = StatusMessage::Idle;
    state.refresh(searcher);
}

fn switch_mode(state: &mut AppState, searcher: &Searcher) {
    state.mode = state.mode.next();
    state.refresh(searcher);
}

fn copy_selected(state: &mut AppState, database: Option<&Database>, with_description: bool) {
    if state.engine.is_ai() {
        return copy_ai(state);
    }
    state.ensure_processes();
    let Some(selected) = state.selected_result() else {
        return;
    };
    let command = &selected.command;
    let resolved = resolve::resolve(command, &state.candidates());
    let text = match with_description {
        true => format!("# {}\n{}", command.description, resolved),
        false => resolved,
    };
    let id = command.id.clone();
    state.status = match copy_to_clipboard(&text) {
        Ok(()) => {
            // Usage is recorded only after a successful copy so failures don't skew ranking.
            if let Some(database) = database {
                let _ = database.record_use(&id);
            }
            StatusMessage::Copied(text)
        }
        Err(error) => StatusMessage::Error(error.to_string()),
    };
    state.should_quit = true;
}

fn copy_ai(state: &mut AppState) {
    let Some(command) = state.ai_command().map(String::from) else {
        state.status = StatusMessage::Info("press Enter to ask, Esc to cancel".into());
        return;
    };
    state.status = match copy_to_clipboard(&command) {
        Ok(()) => StatusMessage::Copied(command),
        Err(error) => StatusMessage::Error(error.to_string()),
    };
    state.should_quit = true;
}
