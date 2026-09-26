mod builtin;
mod database;

pub use builtin::load_builtin;
pub use database::{Database, Usage};

use serde::Deserialize;

#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Danger {
    Low,
    Medium,
    High,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Placeholder {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Command {
    pub id: String,
    pub mode: String,
    pub trigger: String,
    pub command: String,
    pub description: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub context_tags: Vec<String>,
    #[serde(default)]
    pub placeholders: Vec<Placeholder>,
    pub danger_level: Danger,
    pub base_score: f32,
}

impl Command {
    pub fn intent_text(&self) -> String {
        format!("{} {} {}", self.trigger, self.aliases.join(" "), self.id)
    }
}
