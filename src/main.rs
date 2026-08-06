mod cli;
mod history;
mod shell;
mod tui;

use std::error::Error;

use clap::Parser;
use cli::{Cli, Command};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("terminal-history: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    match Cli::parse().command {
        Command::Add(args) => history::add(args).await,
        Command::List(filter) => history::list(filter, None).await,
        Command::Search { query, filter } => history::list(filter, Some(query)).await,
        Command::Recall { prefix, offset } => history::recall(&prefix, offset).await,
        Command::Pick { query } => history::pick(&query).await,
        Command::Init { shell } => shell::print_init(shell),
        Command::Compact => history::compact().await,
    }
}
