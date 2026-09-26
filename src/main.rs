mod app;
mod cli;
mod clipboard;
mod context;
mod knowledge;
mod modes;
mod search;

use anyhow::{bail, Result};
use app::{AppState, StatusMessage};
use clap::Parser;
use cli::Cli;
use knowledge::Database;
use modes::ActiveMode;
use search::Searcher;

fn main() -> Result<()> {
    let cli = Cli::parse();
    if cli.context {
        println!("{}", serde_json::to_string_pretty(&context::detect())?);
        return Ok(());
    }
    run_palette(cli)
}

fn run_palette(cli: Cli) -> Result<()> {
    // History is a ranking nicety; a broken database must never block the palette.
    let database = Database::open_default().ok();
    let history = database.as_ref().and_then(|db| db.usage_map().ok()).unwrap_or_default();
    let searcher = Searcher::new(knowledge::load_builtin()?, history);
    let remembered_mode = database.as_ref().and_then(Database::last_mode).and_then(|id| ActiveMode::from_id(&id));
    let mode = cli.mode.or(remembered_mode).unwrap_or(ActiveMode::for_current_os());
    let context = context::detect();
    let query = cli.query.join(" ");

    if let Some(position) = cli.pick {
        return print_pick(&searcher, &query, &context, mode, position, cli.verbose);
    }
    let state = app::run(AppState::new(query, mode, context, &searcher), &searcher, database.as_ref())?;
    if let Some(database) = &database {
        let _ = database.set_last_mode(state.mode.id());
    }
    report_clipboard_failure(&state);
    Ok(())
}

fn print_pick(searcher: &Searcher, query: &str, context: &context::ContextSnapshot, mode: ActiveMode, position: usize, verbose: bool) -> Result<()> {
    let results = searcher.search(query, context, mode);
    let Some(picked) = position.checked_sub(1).and_then(|index| results.get(index)) else {
        bail!("no result at position {position}");
    };
    if verbose {
        eprintln!("{} score={:.3} {:?}", picked.command.id, picked.score, picked.breakdown);
    }
    println!("{}", picked.command.command);
    Ok(())
}

fn report_clipboard_failure(state: &AppState) {
    let StatusMessage::Error(error) = &state.status else {
        return;
    };
    let command = state.selected_result().map(|result| result.command.command.as_str()).unwrap_or_default();
    eprintln!("Clipboard unavailable: {error}\n{command}");
}
