mod app;
mod cli;
mod clipboard;
mod config;
mod context;
mod engine;
mod knowledge;
mod llm;
mod modes;
mod resolve;
mod search;
mod secrets;
mod setup;

use anyhow::{bail, Result};
use app::{AppState, StatusMessage};
use clap::Parser;
use cli::Cli;
use knowledge::Database;
use modes::ActiveMode;
use search::Searcher;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = config::load()?;
    for warning in config::warnings(&config) {
        eprintln!("config: {warning}");
    }
    if cli.setup {
        return setup::run(&config);
    }
    if cli.context {
        println!("{}", serde_json::to_string_pretty(&context::detect())?);
        return Ok(());
    }
    run_palette(cli, &config)
}

fn run_palette(cli: Cli, config: &config::Config) -> Result<()> {
    // History is a ranking nicety; a broken database must never block the palette.
    let database = Database::open_default().ok();
    let history = database
        .as_ref()
        .and_then(|db| db.usage_map().ok())
        .unwrap_or_default();
    let searcher = Searcher::new(knowledge::load_builtin()?, history, &config.ranking);
    let remembered_mode = database
        .as_ref()
        .and_then(Database::last_mode)
        .and_then(|id| ActiveMode::from_id(&id));
    let mode = cli
        .mode
        .or(remembered_mode)
        .unwrap_or(ActiveMode::for_current_os());
    let engine = cli.engine(config.ai.default_engine);
    let context = context::detect();
    let query = cli.query();

    if engine.is_ai() {
        return answer(config, &context, mode, &query);
    }

    if let Some(position) = cli.pick {
        return print_pick(&searcher, &query, &context, mode, position, cli.verbose);
    }
    let state = app::run(
        AppState::new(query, mode, context, &searcher, engine, config),
        &searcher,
        database.as_ref(),
        config,
    )?;
    if let Some(database) = &database {
        let _ = database.set_last_mode(state.mode.id());
    }
    report_clipboard_failure(&state);
    Ok(())
}

/// Explicitly asked for, so a failure must surface instead of silently degrading.
fn answer(
    config: &config::Config,
    context: &context::ContextSnapshot,
    mode: ActiveMode,
    query: &str,
) -> Result<()> {
    if query.trim().is_empty() {
        bail!("--ai needs a query, e.g. cmdpal --ai \"undo the last commit\"");
    }
    let command = llm::ask(config, query, context, mode)?;
    clipboard::copy_to_clipboard(&command)?;
    println!("{command}");
    Ok(())
}

fn print_pick(
    searcher: &Searcher,
    query: &str,
    context: &context::ContextSnapshot,
    mode: ActiveMode,
    position: usize,
    verbose: bool,
) -> Result<()> {
    let results = searcher.search(query, context, mode);
    let Some(picked) = position.checked_sub(1).and_then(|index| results.get(index)) else {
        bail!("no result at position {position}");
    };
    if verbose {
        eprintln!(
            "{} score={:.3} {:?}",
            picked.command.id, picked.score, picked.breakdown
        );
    }
    // Process facts are subprocesses, so collect them only when this command
    // actually has a placeholder that needs them.
    let processes = if resolve::needs_processes(&picked.command) {
        context::processes::collect(&context.cwd)
    } else {
        context::ProcessContext::default()
    };
    let candidates = resolve::Candidates::new(context, &processes);
    println!("{}", resolve::resolve(&picked.command, &candidates));
    Ok(())
}

fn report_clipboard_failure(state: &AppState) {
    let StatusMessage::Error(error) = &state.status else {
        return;
    };
    let command = state
        .selected_result()
        .map(|result| result.command.command.as_str())
        .unwrap_or_default();
    eprintln!("Clipboard unavailable: {error}\n{command}");
}
