use crate::engine::Engine;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "cmdpal",
    version,
    about = "Terminal command palette: cmdpal [-m <mode>] [query...]"
)]
pub struct Cli {
    /// Start in this mode (default: the mode you last quit in, else the one matching this OS)
    #[arg(short, long, value_enum)]
    pub mode: Option<crate::modes::ActiveMode>,

    /// Answer with the configured LLM instead of the offline catalog
    #[arg(long)]
    pub ai: bool,

    /// Force an engine regardless of configuration
    #[arg(long, value_enum, conflicts_with = "ai")]
    pub engine: Option<Engine>,

    /// Configure the API key, base URL and model, then exit
    #[arg(long)]
    pub setup: bool,

    /// Print the Nth (1-based) result to stdout instead of opening the TUI
    #[arg(long)]
    pub pick: Option<usize>,

    /// With --pick, print the score breakdown to stderr
    #[arg(long)]
    pub verbose: bool,

    /// Print the detected context as JSON and exit
    #[arg(long)]
    pub context: bool,

    /// Search query (the words are joined with spaces)
    pub query: Vec<String>,
}

impl Cli {
    pub fn query(&self) -> String {
        self.query.join(" ")
    }

    pub fn engine(&self, configured: Engine) -> Engine {
        self.engine
            .unwrap_or(if self.ai { Engine::Ai } else { configured })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_after_positionals_still_parse() {
        let cli = Cli::try_parse_from([
            "cmdpal", "-m", "git", "how", "to", "delete", "this", "branch", "--ai",
        ])
        .unwrap();
        assert_eq!(cli.mode, Some(crate::modes::ActiveMode::Git));
        assert!(cli.ai);
        assert_eq!(cli.query(), "how to delete this branch");
        assert_eq!(cli.engine(Engine::Offline), Engine::Ai);
    }

    #[test]
    fn explicit_engine_flag_wins_over_ai_flag() {
        let cli = Cli::try_parse_from(["cmdpal", "--engine", "offline", "--ai"]).unwrap_err();
        assert_eq!(cli.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    #[test]
    fn engine_flag_alone_is_respected() {
        let cli = Cli::try_parse_from(["cmdpal", "--engine", "ai", "list pods"]).unwrap();
        assert_eq!(cli.engine(Engine::Offline), Engine::Ai);
    }

    #[test]
    fn defaults_to_configured_engine() {
        let cli = Cli::try_parse_from(["cmdpal", "list pods"]).unwrap();
        assert_eq!(cli.engine(Engine::Offline), Engine::Offline);
        assert_eq!(cli.engine(Engine::Ai), Engine::Ai);
    }

    #[test]
    fn invalid_mode_is_rejected() {
        assert!(Cli::try_parse_from(["cmdpal", "-m", "npm"]).is_err());
    }

    #[test]
    fn setup_is_a_flag_so_config_stays_searchable() {
        let cli = Cli::try_parse_from(["cmdpal", "config"]).unwrap();
        assert!(!cli.setup);
        assert_eq!(cli.query(), "config");
    }
}
