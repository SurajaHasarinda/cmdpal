use super::{
    render,
    state::{AppState, StatusMessage},
};
use crate::{clipboard::copy_to_clipboard, knowledge::Database, search::Searcher};
use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

const PAGE_SIZE: isize = 10;

pub fn run(
    mut state: AppState,
    searcher: &Searcher,
    database: Option<&Database>,
) -> Result<AppState> {
    let mut terminal =
        ratatui::try_init().context("cannot start the terminal UI (is stdout a terminal?)")?;
    let outcome = (|| -> Result<()> {
        while !state.should_quit {
            terminal.draw(|frame| render::draw(frame, &state))?;
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    handle_key(&mut state, key, searcher, database);
                }
            }
        }
        Ok(())
    })();
    ratatui::restore();
    outcome.map(|()| state)
}

fn handle_key(
    state: &mut AppState,
    key: KeyEvent,
    searcher: &Searcher,
    database: Option<&Database>,
) {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    if state.show_help {
        state.show_help = !matches!(key.code, KeyCode::Esc | KeyCode::Char('q' | '?'));
        return;
    }
    match key.code {
        KeyCode::Esc => state.should_quit = true,
        KeyCode::Char('c' | 'g') if ctrl => state.should_quit = true,
        KeyCode::Char('u') if ctrl => edit_query(state, searcher, String::clear),
        KeyCode::Char('p') if ctrl => state.move_selection(-1),
        KeyCode::Char('n') if ctrl => state.move_selection(1),
        KeyCode::Char('y') if ctrl => copy_selected(state, database, true),
        KeyCode::Up => state.move_selection(-1),
        KeyCode::Down => state.move_selection(1),
        KeyCode::PageUp => state.move_selection(-PAGE_SIZE),
        KeyCode::PageDown => state.move_selection(PAGE_SIZE),
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
    let Some(selected) = state.selected_result() else {
        return;
    };
    let command = &selected.command;
    let text = match with_description {
        true => format!("# {}\n{}", command.description, command.command),
        false => command.command.clone(),
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
