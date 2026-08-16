# terminal-history

[简体中文](README.zh-CN.md)

A cross-shell command history tool powered by the official
[`turso`](https://crates.io/crates/turso) Rust SDK and `clap`. It uses a local
SQLite database by default and can synchronize it with Turso Cloud.

## Features

- Supports Bash, Zsh, Fish, and Nushell.
- Stores every command execution, including duplicates.
- Filters history by the current directory by default.
- Provides prefix navigation with `Up` and `Down` and an inline Ratatui search
  interface with `Ctrl-R`.
- Preserves existing shell hooks and records the command, working directory,
  shell, hostname, exit status, and duration when available.
- Uses local SQLite with optional Turso Cloud synchronization.

## Installation

```sh
cargo install --path .
```

The default database path is
`~/.local/share/terminal-history/history.db`.

## Shell Integration

Add the command for your shell to its configuration file:

```sh
# Bash: ~/.bashrc
eval "$(terminal-history init bash)"

# Zsh: ~/.zshrc
eval "$(terminal-history init zsh)"

# Fish: ~/.config/fish/config.fish
terminal-history init fish | source
```

For Nushell, add the following to `config.nu`:

```nu
terminal-history init nu | save --force ~/.cache/terminal-history.nu
source ~/.cache/terminal-history.nu
```

Regenerate the integration after moving or reinstalling the binary because the
generated script stores its absolute path.

## Usage

```sh
terminal-history list
terminal-history list --limit 100 --cwd /path/to/project
terminal-history list --all
terminal-history search docker --limit 20
terminal-history search docker --all
terminal-history compact
terminal-history --help
```

`list` and `search` only query the current directory by default. Use `--cwd` to
select another directory or `--all` to search across directories. `compact`
reclaims unused SQLite pages without deleting history entries.

## Configuration

Enable Turso Cloud synchronization:

```sh
export TURSO_DATABASE_URL='libsql://your-database.turso.io'
export TURSO_AUTH_TOKEN='your-token'
```

The following environment variables are optional:

| Variable | Purpose |
| --- | --- |
| `HISTORY_DATABASE_PATH` | Override the local SQLite or replica path. |
| `TERMINAL_HISTORY_SEARCH_KEY` | Override the `Ctrl-R` search binding. |
| `TERMINAL_HISTORY_UP_KEY` | Override the history-up binding. |
| `TERMINAL_HISTORY_DOWN_KEY` | Override the history-down binding. |
| `TERMINAL_HISTORY_SELECTOR` | Use an external selector, such as `fzf`. |

Key notation follows the conventions of each shell. The built-in selector does
not require `fzf`.

## Security

Commands synchronized to a configured database may contain passwords, tokens,
or other sensitive values. Avoid putting secrets directly on the command line.
