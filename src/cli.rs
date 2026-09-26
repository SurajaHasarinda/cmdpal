use crate::modes::ActiveMode;
use clap::Parser;

#[derive(Parser)]
#[command(
    name = "cmdpal",
    version,
    about = "Terminal command palette: cmdpal [-m <mode>] [query...]"
)]
pub struct Cli {
    /// Start in this mode (default: the mode you last quit in, else the one matching this OS)
    #[arg(short, long, value_enum)]
    pub mode: Option<ActiveMode>,

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
