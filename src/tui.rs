use std::io::{self, IsTerminal};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Padding, Paragraph},
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
        terminal::enable_raw_mode()?;
        let mut output = io::stderr();
        if let Err(error) = execute!(output, EnterAlternateScreen) {
            terminal::disable_raw_mode()?;
            return Err(error.into());
        }
        let terminal = match Terminal::new(CrosstermBackend::new(output)) {
            Ok(terminal) => terminal,
            Err(error) => {
                let _ = execute!(io::stderr(), LeaveAlternateScreen);
                terminal::disable_raw_mode()?;
                return Err(error.into());
            }
        };
        Ok(Self { terminal })
    }

    fn draw(&mut self, query: &str, matches: &[&Entry], selected: usize) -> Result<()> {
        self.terminal.draw(|frame| {
            let area = frame.area();
            let compact = area.width < 72 || area.height < 12;
            let [header, search, results, help] = Layout::vertical([
                Constraint::Length(if compact { 1 } else { 2 }),
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(if compact { 1 } else { 2 }),
            ])
            .areas(area);

            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled(
                        " TERMINAL ",
                        Style::new()
                            .fg(Color::Black)
                            .bg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        " HISTORY",
                        Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        "  newest first",
                        Style::new()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::ITALIC),
                    ),
                ])),
                header,
            );

            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled(" > ", Style::new().fg(Color::Cyan)),
                    Span::raw(query),
                    Span::styled(" ", Style::new().bg(Color::Gray)),
                ]))
                .block(
                    Block::new()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::new().fg(Color::DarkGray))
                        .title(" Filter ")
                        .title_style(Style::new().fg(Color::Cyan)),
                ),
                search,
            );

            let results_block = Block::new()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::new().fg(Color::DarkGray))
                .title(format!(" History - {} matches ", matches.len()))
                .title_style(Style::new().fg(Color::Yellow))
                .padding(Padding::horizontal(1));
            if matches.is_empty() {
                frame.render_widget(
                    Paragraph::new("No matching commands")
                        .alignment(Alignment::Center)
                        .style(Style::new().fg(Color::DarkGray))
                        .block(results_block),
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
                    .block(results_block)
                    .highlight_symbol("▸ ")
                    .highlight_style(
                        Style::new()
                            .fg(Color::Black)
                            .bg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    );
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
                    Span::styled(" Enter ", key_style()),
                    Span::raw("select  "),
                    Span::styled(" Esc ", key_style()),
                    Span::raw("cancel  "),
                    Span::styled(position, Style::new().fg(Color::DarkGray)),
                ])
            } else {
                Line::from(vec![
                    Span::styled(" ↑↓ ", key_style()),
                    Span::raw("move  "),
                    Span::styled(" Enter ", key_style()),
                    Span::raw("select  "),
                    Span::styled(" Esc ", key_style()),
                    Span::raw("cancel  "),
                    Span::styled(" Ctrl-U ", key_style()),
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
    Style::new().fg(Color::Black).bg(Color::DarkGray)
}

impl Drop for Screen {
    fn drop(&mut self) {
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
        let _ = self.terminal.show_cursor();
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
