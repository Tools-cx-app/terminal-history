use std::io::{self, IsTerminal};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
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

pub fn pick(entries: &[Entry], initial_query: &str) -> Result<Option<Entry>> {
    let entries = sorted(entries);
    if !io::stdin().is_terminal() || !io::stderr().is_terminal() {
        return Ok(newest_match(&entries, initial_query).cloned());
    }

    let mut screen = Screen::open()?;
    let mut query = initial_query.to_owned();
    let mut selected = 0;
    loop {
        let matches = matching(&entries, &query);
        selected = selected.min(matches.len().saturating_sub(1));
        screen.draw(&query, &matches, selected)?;

        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        match (key.modifiers, key.code) {
            (KeyModifiers::CONTROL, KeyCode::Char('c')) => return Ok(None),
            (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
                query.clear();
                selected = 0;
            }
            (_, KeyCode::Enter) => return Ok(matches.get(selected).map(|entry| (*entry).clone())),
            (_, KeyCode::Esc) => return Ok(None),
            (_, KeyCode::Up) => selected = selected.saturating_sub(1),
            (_, KeyCode::Down) => {
                selected = (selected + 1).min(matches.len().saturating_sub(1));
            }
            (_, KeyCode::PageUp) => selected = selected.saturating_sub(10),
            (_, KeyCode::PageDown) => {
                selected = (selected + 10).min(matches.len().saturating_sub(1));
            }
            (_, KeyCode::Home) => selected = 0,
            (_, KeyCode::End) => selected = matches.len().saturating_sub(1),
            (_, KeyCode::Backspace) => {
                query.pop();
                selected = 0;
            }
            (KeyModifiers::NONE | KeyModifiers::SHIFT, KeyCode::Char(character)) => {
                query.push(character);
                selected = 0;
            }
            _ => {}
        }
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
        let terminal = match Terminal::with_options(
            CrosstermBackend::new(output),
            TerminalOptions {
                viewport: Viewport::Inline(height),
            },
        ) {
            Ok(terminal) => terminal,
            Err(error) => {
                let _ = execute!(io::stderr(), cursor::Show);
                terminal::disable_raw_mode()?;
                return Err(error.into());
            }
        };
        Ok(Self { terminal })
    }

    fn draw(&mut self, query: &str, matches: &[&Entry], selected: usize) -> Result<()> {
        self.terminal.draw(|frame| {
            let area = frame.area();
            let compact = area.width < 64;
            let [search, results, help] = Layout::vertical([
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .areas(area);

            let [input, count] = Layout::horizontal([
                Constraint::Min(1),
                Constraint::Length(matches.len().to_string().len() as u16 + 9),
            ])
            .areas(search);
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("history> ", Style::new().fg(Color::Cyan)),
                    Span::raw(query),
                    Span::styled(" ", Style::new().bg(Color::Gray)),
                ])),
                input,
            );
            frame.render_widget(
                Paragraph::new(format!("{} matches", matches.len()))
                    .alignment(Alignment::Right)
                    .style(Style::new().fg(Color::DarkGray)),
                count,
            );

            if matches.is_empty() {
                frame.render_widget(
                    Paragraph::new("No matching commands")
                        .alignment(Alignment::Center)
                        .style(Style::new().fg(Color::DarkGray)),
                    results,
                );
            } else {
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
                let mut state = ListState::default().with_selected(Some(selected));
                frame.render_stateful_widget(list, results, &mut state);
            }

            let position = if matches.is_empty() {
                "0/0".to_owned()
            } else {
                format!("{}/{}", selected + 1, matches.len())
            };
            let help_line = if compact {
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
        let _ = self.terminal.clear();
        let _ = execute!(
            self.terminal.backend_mut(),
            cursor::MoveToColumn(0),
            terminal::Clear(ClearType::FromCursorDown),
            cursor::Show
        );
        let _ = terminal::disable_raw_mode();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
