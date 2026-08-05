# Repository Notes

## Verify Changes

- Run `cargo fmt --check`, `cargo test --locked`, and `cargo clippy --locked --all-targets -- -D warnings`.
- Run one test with `cargo test --locked <test_name>`; current unit tests live beside the code in `src/cli.rs` and `src/history.rs`.
- Validate generated integrations after changing `src/shell.rs`:
  - Bash: `cargo run --quiet -- init bash | bash -n`
  - Nushell: `SCRIPT="$(cargo run --quiet -- init nu)" nu -c '$env.SCRIPT | nu-check'`
- Use `HISTORY_DATABASE_PATH=/tmp/<name>.db` for manual CLI checks. Otherwise commands touch the user's real database under `~/.local/share/terminal-history/`.

## Architecture

- `src/main.rs` only dispatches Clap commands. Keep argument definitions in `src/cli.rs`, Turso access and history behavior in `src/history.rs`, the Ratatui selector in `src/tui.rs`, and generated shell code in `src/shell.rs`.
- `list` and `search` pull Turso Cloud changes before reading; `add` and `recall` deliberately avoid a pull because shell hooks and arrow navigation are latency-sensitive. Writes push when remote sync is configured.
- History is scoped to the exact shell-provided `PWD` by default, preserving symlink paths. `--all` is the explicit cross-directory path.
- Commands are intentionally not unique: every execution is a row. `executed_at` is a unique nanosecond timestamp; `SCHEMA` migrates legacy second timestamps before creating its unique index.
- `pick` uses the built-in Ratatui selector by default. `TERMINAL_HISTORY_SELECTOR` explicitly opts into an external selector; if it cannot start, the newest match is returned.
- The Ratatui inline viewport renders only to stderr; stdout must contain only the selected command because shell widgets capture it with command substitution.

## Change Hazards

- The project uses the official `turso` crate with its `sync` feature, not `libsql`. Keep `Cargo.lock`; `turso` is currently a pre-release dependency.
- Internal keybinding commands are blocked on insert by `is_internal_command` and hidden from existing rows by `HIDE_INTERNAL`. Update both when adding an internal command.
- `init` replaces `__TERMINAL_HISTORY_BIN__` with `current_exe()` so generated hooks work without `PATH`. Moving or reinstalling the binary requires regenerating the shell script.
- The executable is `terminal-history`, not `history`; `history` conflicts with shell built-ins.
- Shell snippets must append to existing hooks/config rather than replace user hooks. Bash does not provide reliable duration data, so its stored duration is intentionally null.
