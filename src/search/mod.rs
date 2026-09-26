mod engine;
mod ranker;

pub use ranker::ScoredCommand;

use crate::{
    context::ContextSnapshot,
    knowledge::{Command, Usage},
    modes::ActiveMode,
};
use engine::{IndexedCommand, SearchEngine};
use ranker::Ranker;
use std::collections::HashMap;

pub struct Searcher {
    commands: Vec<IndexedCommand>,
    history: HashMap<String, Usage>,
    ranker: Ranker,
    engine: SearchEngine,
}

impl Searcher {
    pub fn new(commands: Vec<Command>, history: HashMap<String, Usage>) -> Self {
        let commands = commands.into_iter().map(IndexedCommand::new).collect();
        Self { commands, history, ranker: Ranker::default(), engine: SearchEngine::new() }
    }

    pub fn search(&self, query: &str, context: &ContextSnapshot, mode: ActiveMode) -> Vec<ScoredCommand> {
        self.ranker.rank(query, &self.commands, context, &self.history, &self.engine, mode)
    }
}

