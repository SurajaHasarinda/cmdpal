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
use app::AppState;
use clap::Parser;
use cli::{Cli, Intent};
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
    let context = context::detect();
    let query = cli.query();

    // The invocation shape picks the surface; the engine is never a flag.
    match cli.intent() {
        Intent::Pick(position) => {
            return print_pick(&searcher, &query, &context, mode, position, cli.verbose);
        }
        Intent::Ask => return answer(config, &context, cli.mode, &query),
        Intent::Console => {}
    }

    let state = app::run(
        AppState::new(query, mode, context, &searcher, config),
        &searcher,
        database.as_ref(),
        config,
    )?;
    if let Some(database) = &database {
        let _ = database.set_last_mode(state.mode.id());
    }
    Ok(())
}

fn answer(
    config: &config::Config,
    context: &context::ContextSnapshot,
    mode: Option<ActiveMode>,
    query: &str,
) -> Result<()> {
    let command = llm::ask(config, query, context, mode)?;
    let _ = clipboard::copy_to_clipboard(&command);
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
    // Process facts are subprocesses, so collect them only when needed.
    let processes = if resolve::needs_processes(&picked.command) {
        context::processes::collect(&context.cwd)
    } else {
        context::ProcessContext::default()
    };
    let candidates = resolve::Candidates::new(context, &processes);
    println!("{}", resolve::resolve(&picked.command, &candidates));
    Ok(())
}
