use crate::{
    context::ContextSnapshot,
    modes::ActiveMode,
    search::{ScoredCommand, Searcher},
};

const EMPTY_QUERY_RESULTS: usize = 10;

pub enum StatusMessage {
    Idle,
    Copied(String),
    Error(String),
}

pub struct AppState {
    pub query: String,
    pub results: Vec<ScoredCommand>,
    pub selected: usize,
    pub mode: ActiveMode,
    pub context: ContextSnapshot,
    pub should_quit: bool,
    pub show_help: bool,
    pub status: StatusMessage,
}

impl AppState {
    pub fn new(
        query: String,
        mode: ActiveMode,
        context: ContextSnapshot,
        searcher: &Searcher,
    ) -> Self {
        let mut state = Self {
            query,
            results: Vec::new(),
            selected: 0,
            mode,
            context,
            should_quit: false,
            show_help: false,
            status: StatusMessage::Idle,
        };
        state.refresh(searcher);
        state
    }

    pub fn refresh(&mut self, searcher: &Searcher) {
        self.results = searcher.search(&self.query, &self.context, self.mode);
        if self.query.is_empty() {
            self.results.truncate(EMPTY_QUERY_RESULTS);
        }
        self.selected = self.selected.min(self.results.len().saturating_sub(1));
    }

    pub fn selected_result(&self) -> Option<&ScoredCommand> {
        self.results.get(self.selected)
    }

    pub fn move_selection(&mut self, delta: isize) {
        let last = self.results.len().saturating_sub(1) as isize;
        self.selected = (self.selected as isize + delta).clamp(0, last) as usize;
    }
}
