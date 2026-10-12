mod engine;
mod ranker;

#[cfg(test)]
mod accuracy;

pub use ranker::ScoredCommand;

use crate::{
    config::Ranking,
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
    pub fn new(commands: Vec<Command>, history: HashMap<String, Usage>, ranking: &Ranking) -> Self {
        let commands = commands.into_iter().map(IndexedCommand::new).collect();
        Self {
            commands,
            history,
            ranker: Ranker::from(ranking),
            engine: SearchEngine::new(),
        }
    }

    pub fn search(
        &self,
        query: &str,
        context: &ContextSnapshot,
        mode: ActiveMode,
    ) -> Vec<ScoredCommand> {
        self.ranker.rank(
            query,
            &self.commands,
            context,
            &self.history,
            &self.engine,
            mode,
        )
    }
}
