use crate::{
    config::Config,
    context::ContextSnapshot,
    engine::Engine,
    modes::ActiveMode,
    search::{ScoredCommand, Searcher},
};

const EMPTY_QUERY_RESULTS: usize = 10;

pub enum StatusMessage {
    Idle,
    Copied(String),
    Error(String),
    Info(String),
}

pub enum AiState {
    Idle,
    Pending,
    Ready(String),
    Failed(String),
}

pub struct AppState {
    pub query: String,
    pub results: Vec<ScoredCommand>,
    pub selected: usize,
    pub mode: ActiveMode,
    pub engine: Engine,
    pub context: ContextSnapshot,
    pub should_quit: bool,
    pub show_help: bool,
    pub status: StatusMessage,
    pub ai: AiState,
    pub model: String,
}

impl AppState {
    pub fn new(
        query: String,
        mode: ActiveMode,
        context: ContextSnapshot,
        searcher: &Searcher,
        engine: Engine,
        config: &Config,
    ) -> Self {
        let mut state = Self {
            query,
            results: Vec::new(),
            selected: 0,
            mode,
            engine,
            context,
            should_quit: false,
            show_help: false,
            status: StatusMessage::Idle,
            ai: AiState::Idle,
            model: config.ai.model.clone(),
        };
        state.refresh(searcher);
        state
    }

    pub fn refresh(&mut self, searcher: &Searcher) {
        if self.engine.is_ai() {
            self.results.clear();
            self.selected = 0;
            return;
        }
        self.results = searcher.search(&self.query, &self.context, self.mode);
        if self.query.is_empty() {
            self.results.truncate(EMPTY_QUERY_RESULTS);
        }
        self.selected = self.selected.min(self.results.len().saturating_sub(1));
    }

    pub fn toggle_engine(&mut self, searcher: &Searcher) {
        self.engine = self.engine.toggled();
        self.ai = AiState::Idle;
        self.status = StatusMessage::Info(format!("engine: {}", self.engine.label()));
        self.refresh(searcher);
    }

    pub fn apply_ai(&mut self, outcome: Result<String, String>) {
        match outcome {
            Ok(command) => {
                self.ai = AiState::Ready(command);
                self.status = StatusMessage::Info("AI answer ready — Enter copies".into());
            }
            Err(error) => {
                self.ai = AiState::Failed(error.clone());
                self.status = StatusMessage::Error(error);
            }
        }
    }

    pub fn ai_command(&self) -> Option<&str> {
        match &self.ai {
            AiState::Ready(command) => Some(command),
            _ => None,
        }
    }

    pub fn is_ai_pending(&self) -> bool {
        matches!(self.ai, AiState::Pending)
    }

    pub fn cancel_ai(&mut self) {
        if self.is_ai_pending() {
            self.ai = AiState::Idle;
            self.status = StatusMessage::Info("AI request cancelled".into());
        }
    }

    pub fn begin_ai(&mut self) {
        self.ai = AiState::Pending;
        self.status = StatusMessage::Idle;
    }

    pub fn selected_result(&self) -> Option<&ScoredCommand> {
        self.results.get(self.selected)
    }

    pub fn move_selection(&mut self, delta: isize) {
        let last = self.results.len().saturating_sub(1) as isize;
        self.selected = (self.selected as isize + delta).clamp(0, last) as usize;
    }
}
