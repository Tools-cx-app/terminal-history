use std::{
    io::{self, IsTerminal, Write},
    sync::mpsc::{Receiver, TryRecvError},
    time::{Duration, Instant},
};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{self, ClearType},
};
use ratatui::{
    Terminal, TerminalOptions, Viewport,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph},
};

use crate::Result;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub command: String,
    pub executed_at: i64,
    pub display_time: String,
}

pub fn pick(
    receiver: Receiver<std::result::Result<Vec<Entry>, String>>,
    initial_query: &str,
) -> Result<Option<Entry>> {
    if !io::stdin().is_terminal() || !io::stderr().is_terminal() {
        let entries = receiver.recv()?.map_err(io::Error::other)?;
        return Ok(newest_match(&entries, initial_query).cloned());
    }

    let mut screen = Screen::open()?;
    let mut picker = Picker::new(initial_query);
    let started = Instant::now();
    loop {
        match receiver.try_recv() {
            Ok(Ok(entries)) if entries.is_empty() => return Ok(None),
            Ok(result) => picker.finish_loading(result),
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) if matches!(picker.state, LoadState::Loading) => {
                picker.finish_loading(Err("history loader stopped".into()));
            }
            Err(TryRecvError::Disconnected) => {}
        }

        screen.draw(&picker, (started.elapsed().as_millis() / 80) as usize)?;
        if !event::poll(Duration::from_millis(80))? {
            continue;
        }

        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        match picker.handle_key(key) {
            Action::Continue => {}
            Action::Select(entry) => return Ok(entry),
            Action::Cancel => return Ok(None),
        }
    }
}

enum LoadState {
    Loading,
    Ready(Vec<Entry>),
    Failed(String),
}

#[derive(Debug, PartialEq, Eq)]
enum Action {
    Continue,
    Select(Option<Entry>),
    Cancel,
}

struct Picker {
    state: LoadState,
    query: String,
    selected: usize,
}

impl Picker {
    fn new(initial_query: &str) -> Self {
        Self {
            state: LoadState::Loading,
            query: initial_query.to_owned(),
            selected: 0,
        }
    }

    fn finish_loading(&mut self, result: std::result::Result<Vec<Entry>, String>) {
        self.state = match result {
            Ok(entries) => LoadState::Ready(sorted(&entries)),
            Err(error) => LoadState::Failed(error),
        };
        self.selected = 0;
    }

    fn matches(&self) -> Vec<&Entry> {
        match &self.state {
            LoadState::Ready(entries) => matching(entries, &self.query),
            _ => Vec::new(),
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> Action {
        if matches!(
            (key.modifiers, key.code),
            (KeyModifiers::CONTROL, KeyCode::Char('c')) | (_, KeyCode::Esc)
        ) {
            return Action::Cancel;
        }
        if matches!(self.state, LoadState::Failed(_)) {
            return Action::Continue;
        }

        let match_count = self.matches().len();
        match (key.modifiers, key.code) {
            (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
                self.query.clear();
                self.selected = 0;
            }
            (_, KeyCode::Enter) if matches!(self.state, LoadState::Ready(_)) => {
                return Action::Select(
                    self.matches()
                        .get(self.selected)
                        .map(|entry| (*entry).clone()),
                );
            }
            (_, KeyCode::Up) if matches!(self.state, LoadState::Ready(_)) => {
                self.selected = self.selected.saturating_sub(1);
            }
            (_, KeyCode::Down) if matches!(self.state, LoadState::Ready(_)) => {
                self.selected = (self.selected + 1).min(match_count.saturating_sub(1));
            }
            (_, KeyCode::PageUp) if matches!(self.state, LoadState::Ready(_)) => {
                self.selected = self.selected.saturating_sub(10);
            }
            (_, KeyCode::PageDown) if matches!(self.state, LoadState::Ready(_)) => {
                self.selected = (self.selected + 10).min(match_count.saturating_sub(1));
            }
            (_, KeyCode::Home) if matches!(self.state, LoadState::Ready(_)) => self.selected = 0,
            (_, KeyCode::End) if matches!(self.state, LoadState::Ready(_)) => {
                self.selected = match_count.saturating_sub(1);
            }
            (_, KeyCode::Backspace) => {
                self.query.pop();
                self.selected = 0;
            }
            (KeyModifiers::NONE | KeyModifiers::SHIFT, KeyCode::Char(character)) => {
                self.query.push(character);
                self.selected = 0;
            }
            _ => {}
        }
        Action::Continue
    }
}

pub fn newest_match<'a>(entries: &'a [Entry], query: &str) -> Option<&'a Entry> {
    matching(entries, query)
        .into_iter()
        .max_by_key(|entry| entry.executed_at)
}

fn sorted(entries: &[Entry]) -> Vec<Entry> {
    let mut entries = entries.to_vec();
    entries.sort_by_key(|entry| std::cmp::Reverse(entry.executed_at));
    entries
}

fn matching<'a>(entries: &'a [Entry], query: &str) -> Vec<&'a Entry> {
    let words: Vec<_> = query.split_whitespace().map(str::to_lowercase).collect();
    entries
        .iter()
        .filter(|entry| {
            let command = entry.command.to_lowercase();
            words.iter().all(|word| command.contains(word))
        })
        .collect()
}

struct Screen {
    terminal: Terminal<CrosstermBackend<io::Stderr>>,
}

impl Screen {
    fn open() -> Result<Self> {
        let (_, terminal_height) = terminal::size()?;
        terminal::enable_raw_mode()?;
        let mut output = io::stderr();
        if let Err(error) = execute!(output, cursor::MoveToNextLine(1), cursor::Hide) {
            terminal::disable_raw_mode()?;
            return Err(error.into());
        }
        let height = (terminal_height * 2 / 5).clamp(5, 14);
        let terminal = match with_terminal_stdout(|| {
            Terminal::with_options(
                CrosstermBackend::new(output),
                TerminalOptions {
                    viewport: Viewport::Inline(height),
                },
            )
        }) {
            Ok(terminal) => terminal,
            Err(error) => {
                let _ = execute!(io::stderr(), cursor::Show);
                terminal::disable_raw_mode()?;
                return Err(error.into());
            }
        };
        Ok(Self { terminal })
    }

    fn draw(&mut self, picker: &Picker, spinner: usize) -> Result<()> {
        self.terminal.draw(|frame| {
            let area = frame.area();
            let compact = area.width < 64;
            let matches = picker.matches();
            let status = match &picker.state {
                LoadState::Loading => "loading".to_owned(),
                LoadState::Failed(_) => "error".to_owned(),
                LoadState::Ready(_) => format!("{} matches", matches.len()),
            };
            let [search, results, help] = Layout::vertical([
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .areas(area);

            let [input, count] =
                Layout::horizontal([Constraint::Min(1), Constraint::Length(status.len() as u16)])
                    .areas(search);
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("> ", Style::new().fg(Color::Cyan)),
                    Span::raw(&picker.query),
                    Span::styled(" ", Style::new().bg(Color::Gray)),
                ])),
                input,
            );
            frame.render_widget(
                Paragraph::new(status)
                    .alignment(Alignment::Right)
                    .style(Style::new().fg(Color::DarkGray)),
                count,
            );

            match &picker.state {
                LoadState::Loading => frame.render_widget(
                    Paragraph::new(format!(
                        "{} Loading history...",
                        ['|', '/', '-', '\\'][spinner % 4]
                    ))
                    .alignment(Alignment::Center)
                    .style(Style::new().fg(Color::DarkGray)),
                    results,
                ),
                LoadState::Failed(error) => frame.render_widget(
                    Paragraph::new(format!("Failed to load history: {error}"))
                        .alignment(Alignment::Center)
                        .style(Style::new().fg(Color::Red)),
                    results,
                ),
                LoadState::Ready(_) if matches.is_empty() => frame.render_widget(
                    Paragraph::new("No matching commands")
                        .alignment(Alignment::Center)
                        .style(Style::new().fg(Color::DarkGray)),
                    results,
                ),
                LoadState::Ready(_) => {
                    let show_time = results.width >= 54;
                    let items = matches.iter().map(|entry| {
                        let command = entry.command.replace('\n', " ");
                        let line = if show_time {
                            Line::from(vec![
                                Span::styled(
                                    format!("{}  ", entry.display_time),
                                    Style::new().fg(Color::DarkGray),
                                ),
                                Span::raw(command),
                            ])
                        } else {
                            Line::raw(command)
                        };
                        ListItem::new(line).style(Style::new().fg(Color::White))
                    });
                    let list = List::new(items)
                        .highlight_symbol("▸ ")
                        .highlight_style(Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD));
                    let mut state = ListState::default().with_selected(Some(picker.selected));
                    frame.render_stateful_widget(list, results, &mut state);
                }
            }

            let position = if matches.is_empty() {
                "0/0".to_owned()
            } else {
                format!("{}/{}", picker.selected + 1, matches.len())
            };
            let help_line = if matches!(picker.state, LoadState::Failed(_)) {
                Line::from(vec![Span::styled("esc", key_style()), Span::raw(" cancel")])
            } else if matches!(picker.state, LoadState::Loading) {
                Line::from(vec![
                    Span::styled("ctrl-u", key_style()),
                    Span::raw(" clear  "),
                    Span::styled("esc", key_style()),
                    Span::raw(" cancel"),
                ])
            } else if compact {
                Line::from(vec![
                    Span::styled("↑↓", key_style()),
                    Span::raw(" move  "),
                    Span::styled("enter", key_style()),
                    Span::raw(" select  "),
                    Span::styled("esc", key_style()),
                    Span::raw(" cancel  "),
                    Span::styled(position, Style::new().fg(Color::DarkGray)),
                ])
            } else {
                Line::from(vec![
                    Span::styled("↑↓/pgup/pgdn", key_style()),
                    Span::raw("move  "),
                    Span::styled("enter", key_style()),
                    Span::raw("select  "),
                    Span::styled("esc", key_style()),
                    Span::raw("cancel  "),
                    Span::styled("ctrl-u", key_style()),
                    Span::raw("clear  "),
                    Span::styled(position, Style::new().fg(Color::DarkGray)),
                ])
            };
            frame.render_widget(Paragraph::new(help_line).alignment(Alignment::Center), help);
        })?;
        Ok(())
    }
}

fn key_style() -> Style {
    Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD)
}

impl Drop for Screen {
    fn drop(&mut self) {
        let _ = with_terminal_stdout(|| self.terminal.clear());
        let _ = execute!(
            self.terminal.backend_mut(),
            cursor::MoveToColumn(0),
            terminal::Clear(ClearType::FromCursorDown),
            cursor::Show
        );
        let _ = terminal::disable_raw_mode();
    }
}

#[cfg(unix)]
fn with_terminal_stdout<T>(f: impl FnOnce() -> T) -> T {
    // Crossterm sends cursor-position queries to stdout, which Ctrl-R captures.
    struct RestoreStdout(i32);

    impl Drop for RestoreStdout {
        fn drop(&mut self) {
            unsafe {
                libc::dup2(self.0, libc::STDOUT_FILENO);
                libc::close(self.0);
            }
        }
    }

    let _ = io::stdout().flush();
    let saved_stdout = unsafe { libc::dup(libc::STDOUT_FILENO) };
    if saved_stdout < 0 || unsafe { libc::dup2(libc::STDERR_FILENO, libc::STDOUT_FILENO) } < 0 {
        if saved_stdout >= 0 {
            unsafe { libc::close(saved_stdout) };
        }
        return f();
    }
    let _restore = RestoreStdout(saved_stdout);
    f()
}

#[cfg(not(unix))]
fn with_terminal_stdout<T>(f: impl FnOnce() -> T) -> T {
    f()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn entry(command: &str, executed_at: i64) -> Entry {
        Entry {
            command: command.into(),
            executed_at,
            display_time: "08-05 12:00:00".into(),
        }
    }

    #[test]
    fn sorts_by_timestamp_and_filters_case_insensitive_words() {
        let entries = vec![
            entry("git status", 10),
            entry("cargo test", 30),
            entry("git log", 20),
        ];
        let sorted = sorted(&entries);
        assert_eq!(
            sorted
                .iter()
                .map(|entry| entry.executed_at)
                .collect::<Vec<_>>(),
            [30, 20, 10]
        );
        assert_eq!(matching(&sorted, "LOG git"), vec![&sorted[1]]);
        assert_eq!(newest_match(&entries, "git"), Some(&entries[2]));
    }

    #[test]
    fn edits_query_while_loading_and_ignores_enter() {
        let mut picker = Picker::new("git");

        assert_eq!(picker.handle_key(key(KeyCode::Char(' '))), Action::Continue);
        assert_eq!(picker.handle_key(key(KeyCode::Char('l'))), Action::Continue);
        assert_eq!(picker.query, "git l");
        assert_eq!(picker.handle_key(key(KeyCode::Enter)), Action::Continue);

        picker.finish_loading(Ok(vec![entry("git status", 20), entry("git log", 10)]));
        assert_eq!(picker.matches()[0].command, "git log");
    }

    #[test]
    fn records_load_failure_and_allows_cancel() {
        let mut picker = Picker::new("");
        picker.finish_loading(Err("offline".into()));

        assert!(matches!(
            &picker.state,
            LoadState::Failed(error) if error == "offline"
        ));
        assert_eq!(picker.handle_key(key(KeyCode::Esc)), Action::Cancel);
    }
}
