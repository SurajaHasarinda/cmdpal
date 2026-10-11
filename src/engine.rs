use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Engine {
    #[default]
    Offline,
    Ai,
}

impl Engine {
    pub const fn is_ai(self) -> bool {
        matches!(self, Engine::Ai)
    }

    pub const fn toggled(self) -> Self {
        match self {
            Engine::Offline => Engine::Ai,
            Engine::Ai => Engine::Offline,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Engine::Offline => "offline",
            Engine::Ai => "ai",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggling_round_trips() {
        assert_eq!(Engine::Offline.toggled().toggled(), Engine::Offline);
        assert!(Engine::Ai.is_ai());
        assert!(!Engine::Offline.is_ai());
    }

    #[test]
    fn parses_from_cli_and_defaults_to_offline() {
        assert_eq!(Engine::from_str("ai", true).unwrap(), Engine::Ai);
        assert_eq!(Engine::from_str("offline", true).unwrap(), Engine::Offline);
        assert_eq!(Engine::default(), Engine::Offline);
    }

    #[test]
    fn label_matches_variants() {
        assert_eq!(Engine::Offline.label(), "offline");
        assert_eq!(Engine::Ai.label(), "ai");
    }
}
