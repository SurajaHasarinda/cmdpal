use super::engine::{tokenize, IndexedCommand, SearchEngine};
use crate::{
    config::Ranking,
    context::ContextSnapshot,
    knowledge::{Command, Usage},
    modes::ActiveMode,
};
use chrono::Utc;
use std::collections::HashMap;

const MAX_RESULTS: usize = 50;
const RECENCY_DECAY_DAYS: f32 = 30.0;
const USAGE_SATURATION: f32 = 100.0;

pub struct Ranker {
    pub tag_weight: f32,
    pub usage_weight: f32,
    pub recency_weight: f32,
    pub mode_boost: f32,
    pub match_weight: f32,
    pub other_mode_demotion: f32,
}

impl From<&Ranking> for Ranker {
    fn from(ranking: &Ranking) -> Self {
        Self {
            tag_weight: ranking.tag_weight,
            usage_weight: ranking.usage_weight,
            recency_weight: ranking.recency_weight,
            mode_boost: ranking.mode_boost,
            match_weight: ranking.match_weight,
            other_mode_demotion: ranking.demote_other_modes,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ScoreBreakdown {
    pub base: f32,
    pub tags: f32,
    pub usage: f32,
    pub recency: f32,
    pub mode: f32,
    pub matched: f32,
}

impl ScoreBreakdown {
    fn total(&self) -> f32 {
        self.base + self.tags + self.usage + self.recency + self.mode + self.matched
    }
}

pub struct ScoredCommand {
    pub command: Command,
    pub score: f32,
    pub breakdown: ScoreBreakdown,
}

impl Ranker {
    pub fn rank(
        &self,
        query: &str,
        commands: &[IndexedCommand],
        context: &ContextSnapshot,
        history: &HashMap<String, Usage>,
        engine: &SearchEngine,
        active_mode: ActiveMode,
    ) -> Vec<ScoredCommand> {
        let tokens = tokenize(query);
        let mut ranked: Vec<_> = commands
            .iter()
            .filter_map(|indexed| {
                self.score_command(
                    indexed,
                    &tokens,
                    context,
                    history.get(&indexed.command.id),
                    engine,
                    active_mode,
                )
            })
            .collect();
        ranked.sort_by(|a, b| b.score.total_cmp(&a.score));
        ranked.truncate(MAX_RESULTS);
        ranked
    }

    fn score_command(
        &self,
        indexed: &IndexedCommand,
        tokens: &[String],
        context: &ContextSnapshot,
        usage: Option<&Usage>,
        engine: &SearchEngine,
        active_mode: ActiveMode,
    ) -> Option<ScoredCommand> {
        let matched = engine.match_score(indexed, tokens)?;
        let command = &indexed.command;
        let in_active_mode = command.mode == active_mode.id();
        let breakdown = ScoreBreakdown {
            base: command.base_score,
            tags: self.tag_weight * tag_relevance(indexed, tokens, context),
            usage: self.usage_weight * usage.map_or(0.0, usage_signal),
            recency: self.recency_weight * usage.map_or(0.0, recency_signal),
            mode: if in_active_mode { self.mode_boost } else { 0.0 },
            matched: self.match_weight * matched,
        };
        let demotion = if in_active_mode {
            1.0
        } else {
            self.other_mode_demotion
        };
        Some(ScoredCommand {
            command: command.clone(),
            score: breakdown.total() * demotion,
            breakdown,
        })
    }
}

fn tag_relevance(indexed: &IndexedCommand, tokens: &[String], context: &ContextSnapshot) -> f32 {
    let query_hits = tokens.iter().filter(|token| indexed.has_tag(token)).count();
    let context_hits = indexed
        .command
        .context_tags
        .iter()
        .filter(|tag| context.tags.contains(*tag))
        .count();
    ((query_hits + context_hits) as f32 / tokens.len().max(1) as f32).min(1.0)
}

fn usage_signal(usage: &Usage) -> f32 {
    ((1.0 + usage.use_count as f32).ln() / (1.0 + USAGE_SATURATION).ln()).min(1.0)
}

fn recency_signal(usage: &Usage) -> f32 {
    let age_days = (Utc::now() - usage.last_used).num_seconds().max(0) as f32 / 86_400.0;
    (-age_days / RECENCY_DECAY_DAYS).exp()
}
