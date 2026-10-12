use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "cmdpal",
    version,
    about = "Terminal command palette: cmdpal [-m <mode>] [query...]"
)]
pub struct Cli {
    /// Start the console in this mode (default: the last one you used, else your OS)
    #[arg(short, long, value_enum)]
    pub mode: Option<crate::modes::ActiveMode>,

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

    /// Search query (the words are joined with spaces). A query means "ask the model and exit"
    pub query: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Console,
    Ask,
    Pick(usize),
}

impl Cli {
    pub fn query(&self) -> String {
        self.query.join(" ")
    }

    /// `--pick` wins over a query, keeping scripted lookups on the offline catalog.
    pub fn intent(&self) -> Intent {
        match self.pick {
            Some(position) => Intent::Pick(position),
            None if self.query().trim().is_empty() => Intent::Console,
            None => Intent::Ask,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modes::ActiveMode;

    #[test]
    fn a_bare_invocation_opens_the_console() {
        let cli = Cli::try_parse_from(["cmdpal"]).unwrap();
        assert_eq!(cli.intent(), Intent::Console);
    }

    #[test]
    fn flags_alone_still_open_the_console() {
        for argv in [
            vec!["cmdpal", "-m", "docker"],
            vec!["cmdpal", "--verbose"],
            vec!["cmdpal", "--mode", "git"],
        ] {
            assert_eq!(Cli::try_parse_from(argv).unwrap().intent(), Intent::Console);
        }
    }

    #[test]
    fn a_query_means_ask_the_model() {
        let cli = Cli::try_parse_from(["cmdpal", "find", "the", "running", "ports"]).unwrap();
        assert_eq!(cli.query(), "find the running ports");
        assert_eq!(cli.intent(), Intent::Ask);
    }

    #[test]
    fn a_query_with_a_mode_means_ask_the_model() {
        let cli = Cli::try_parse_from(["cmdpal", "-m", "docker", "list", "containers"]).unwrap();
        assert_eq!(cli.mode, Some(ActiveMode::Docker));
        assert_eq!(cli.intent(), Intent::Ask);
    }

    #[test]
    fn pick_wins_over_a_query() {
        let cli = Cli::try_parse_from(["cmdpal", "--pick", "1", "delete", "directory"]).unwrap();
        assert_eq!(cli.intent(), Intent::Pick(1));
        assert_eq!(cli.query(), "delete directory");
    }

    #[test]
    fn pick_without_a_query_still_picks() {
        let cli = Cli::try_parse_from(["cmdpal", "--pick", "3"]).unwrap();
        assert_eq!(cli.intent(), Intent::Pick(3));
    }

    #[test]
    fn whitespace_only_query_still_opens_the_console() {
        let cli = Cli::try_parse_from(["cmdpal", " "]).unwrap();
        assert_eq!(cli.intent(), Intent::Console);
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
