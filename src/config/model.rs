use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const GOOGLE_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta/openai/";

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Config {
    pub ai: Ai,
    pub context: Context,
    pub ranking: Ranking,
    #[serde(flatten)]
    pub unknown: BTreeMap<String, toml::Value>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct Ai {
    pub base_url: String,
    pub model: String,
    pub timeout_secs: u64,
    pub max_tokens: u32,
    pub temperature: f32,
    pub system_prompt: String,
}

impl Default for Ai {
    fn default() -> Self {
        Self {
            base_url: GOOGLE_BASE_URL.into(),
            model: String::new(),
            timeout_secs: 20,
            max_tokens: 256,
            // Command generation must be reproducible for a given context.
            temperature: 0.0,
            system_prompt: String::new(),
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct Context {
    pub include_fs: bool,
    pub include_git: bool,
    pub include_kube: bool,
    pub include_docker: bool,
    pub max_depth: usize,
    pub max_files: usize,
    pub max_bytes: usize,
    pub provider_timeout_ms: u64,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            include_fs: true,
            include_git: true,
            include_kube: true,
            include_docker: true,
            max_depth: 3,
            max_files: 200,
            max_bytes: 16 * 1024,
            provider_timeout_ms: 250,
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct Ranking {
    pub tag_weight: f32,
    pub usage_weight: f32,
    pub recency_weight: f32,
    pub mode_boost: f32,
    pub match_weight: f32,
    pub demote_other_modes: f32,
}

impl Default for Ranking {
    fn default() -> Self {
        Self {
            tag_weight: 0.4,
            usage_weight: 0.3,
            recency_weight: 0.2,
            mode_boost: 0.5,
            // Match quality must outweigh base popularity, but stay low enough that usage history still counts.
            match_weight: 3.0,
            demote_other_modes: 0.3,
        }
    }
}

impl Config {
    pub fn clamp(&mut self) {
        self.ai.timeout_secs = self.ai.timeout_secs.clamp(1, 300);
        self.ai.max_tokens = self.ai.max_tokens.clamp(16, 8192);
        self.ai.temperature = self.ai.temperature.clamp(0.0, 2.0);
        self.context.max_depth = self.context.max_depth.clamp(1, 8);
        self.context.max_files = self.context.max_files.clamp(1, 5000);
        self.context.max_bytes = self.context.max_bytes.clamp(1024, 512 * 1024);
        self.context.provider_timeout_ms = self.context.provider_timeout_ms.clamp(50, 5000);
        self.ranking.tag_weight = self.ranking.tag_weight.clamp(0.0, 10.0);
        self.ranking.usage_weight = self.ranking.usage_weight.clamp(0.0, 10.0);
        self.ranking.recency_weight = self.ranking.recency_weight.clamp(0.0, 10.0);
        self.ranking.mode_boost = self.ranking.mode_boost.clamp(0.0, 10.0);
        self.ranking.match_weight = self.ranking.match_weight.clamp(0.0, 10.0);
        self.ranking.demote_other_modes = self.ranking.demote_other_modes.clamp(0.0, 1.0);
        if !self.ai.base_url.is_empty() && !self.ai.base_url.ends_with('/') {
            self.ai.base_url.push('/');
        }
    }
}
