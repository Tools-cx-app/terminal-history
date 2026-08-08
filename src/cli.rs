use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(version, about = "Terminal history backed by Turso")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Insert a history entry.
    Add(AddArgs),
    /// List history from the current directory.
    List(FilterArgs),
    /// Search commands from the current directory.
    Search {
        query: String,
        #[command(flatten)]
        filter: FilterArgs,
    },
    /// Return one recent command matching a prefix.
    #[command(hide = true)]
    Recall {
        #[arg(long, default_value = "")]
        prefix: String,
        #[arg(long, default_value_t = 0, value_parser = clap::value_parser!(i64).range(0..))]
        offset: i64,
    },
    /// Return matching commands for shell integration menus.
    #[command(hide = true)]
    Candidates {
        #[arg(long, default_value = "")]
        prefix: String,
    },
    /// Interactively select a command.
    #[command(hide = true)]
    Pick {
        #[arg(long, default_value = "")]
        query: String,
    },
    /// Print shell integration code.
    Init { shell: Shell },
    /// Reclaim unused SQLite pages.
    Compact,
}

#[derive(Args)]
pub struct AddArgs {
    #[arg(long)]
    pub command: String,
    #[arg(long)]
    pub cwd: Option<PathBuf>,
    #[arg(long, default_value = "unknown")]
    pub shell: String,
    #[arg(long)]
    pub hostname: Option<String>,
    #[arg(long)]
    pub status: Option<i64>,
    #[arg(long)]
    pub duration: Option<i64>,
    #[arg(long, hide = true)]
    pub timestamp: Option<i64>,
}

#[derive(Args)]
pub struct FilterArgs {
    /// Maximum number of entries.
    #[arg(short, long, default_value_t = 30, value_parser = clap::value_parser!(i64).range(1..=1000))]
    pub limit: i64,
    /// Filter by this directory instead of the current directory.
    #[arg(long, conflicts_with = "all")]
    pub cwd: Option<PathBuf>,
    /// Include entries from every directory.
    #[arg(short, long)]
    pub all: bool,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
    #[value(alias = "nushell")]
    Nu,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_filters_current_directory_by_default() {
        let cli = Cli::try_parse_from(["terminal-history", "list"]).unwrap();
        let Command::List(filter) = cli.command else {
            unreachable!()
        };
        assert!(!filter.all);
        assert!(filter.cwd.is_none());
        assert_eq!(filter.limit, 30);
    }
}
