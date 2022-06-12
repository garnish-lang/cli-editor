use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::{Color, Style};
use tui::text::Span;
use tui::widgets::{Block, Borders};

use crate::splits::UserSplits;
use crate::{AppState, EditorFrame};

pub fn render_split(split: usize, app: &AppState, frame: &mut EditorFrame, chunk: Rect) {
    match app.splits.get(split) {
        None => (), // error
        Some(split) => {
            // calculate child width
            let total = match split.direction {
                Direction::Horizontal => chunk.width,
                Direction::Vertical => chunk.height,
            };

            let lengths = if split.panels.len() > 0 {
                let part_size = total / split.panels.len() as u16;
                let mut remaining = total;

                let mut lengths: Vec<Constraint> = split
                    .panels
                    .iter()
                    .take(split.panels.len() - 1)
                    .map(|s| {
                        let l = match s {
                            UserSplits::Panel(index) => match app.panels.get(*index) {
                                Some((_, panel)) => {
                                    if panel.get_length() == 0 {
                                        part_size
                                    } else {
                                        panel.get_length()
                                    }
                                }
                                None => part_size,
                            },
                            UserSplits::Split(_) => part_size,
                        };

                        remaining -= l;
                        Constraint::Length(l)
                    })
                    .collect();

                lengths.push(Constraint::Length(remaining));

                lengths
            } else {
                vec![]
            };

            let chunks = Layout::default()
                .direction(split.direction.clone())
                .constraints(lengths)
                .split(chunk);

            // loop through children and render
            for (child, chunk) in split.panels.iter().zip(chunks) {
                match child {
                    UserSplits::Panel(panel_i) => match app.panels.get(*panel_i) {
                        None => (), // error
                        Some((_, panel)) => {
                            let is_active = *panel_i == app.active_panel;
                            if is_active {
                                let (x, y) = panel.get_cursor(&chunk);
                                frame.set_cursor(x, y);
                            }

                            let title = panel.get_title();

                            let block = Block::default()
                                .title(Span::styled(title, Style::default().fg(Color::White)))
                                .borders(Borders::ALL)
                                .border_style(Style::default().fg(match is_active {
                                    true => Color::Green,
                                    false => Color::White,
                                }));

                            panel.make_widget(frame, chunk, is_active, block);
                        }
                    },
                    UserSplits::Split(split_index) => {
                        match app.splits.get(*split_index) {
                            None => (), // error
                            Some(_) => render_split(*split_index, app, frame, chunk),
                        }
                    }
                }
            }
        }
    }
}
