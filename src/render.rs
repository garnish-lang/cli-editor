use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{Block, Borders};

use crate::splits::UserSplits;
use crate::{AppState, EditorFrame};

pub fn render_split(split: usize, app: &AppState, frame: &mut EditorFrame, chunk: Rect) {
    match app.get_split(split) {
        None => (), // error
        Some(split) => {
            // calculate child width
            let total = match split.direction {
                Direction::Horizontal => chunk.width,
                Direction::Vertical => chunk.height,
            };

            let active_panels = split
                .panels
                .iter()
                .filter(|split| match split {
                    UserSplits::Split(_) => true,
                    UserSplits::Panel(panel_index) => match app.get_panel(*panel_index) {
                        Some(lp) => lp.panel().get_active() && lp.panel().visible(),
                        None => false,
                    },
                })
                .collect::<Vec<&UserSplits>>();

            let lengths = if active_panels.len() > 0 {
                let (fixed_count, fixed_total) = match active_panels
                    .iter()
                    .map(|split| match split {
                        UserSplits::Split(_) => (0, 0),
                        UserSplits::Panel(panel_index) => match app.get_panel(*panel_index) {
                            Some(lp) => match lp.panel().get_length() {
                                0 => (0, 0),
                                n => (1, n),
                            },
                            None => (0, 0),
                        },
                    })
                    .reduce(|total, item| (total.0 + item.0, total.1 + item.1))
                {
                    Some(v) => v,
                    None => (0, 0),
                };

                let dynamic_count = active_panels.len() - fixed_count;
                let mut remaining = total - fixed_total;
                let part_size = if dynamic_count == 0 {
                    remaining
                } else {
                    remaining / dynamic_count as u16
                };

                let mut lengths: Vec<Constraint> = active_panels
                    .iter()
                    .take(active_panels.len() - 1)
                    .map(|s| {
                        let l = match s {
                            UserSplits::Panel(index) => match app.get_panel(*index) {
                                Some(lp) => {
                                    if lp.panel().get_length() == 0 {
                                        part_size
                                    } else {
                                        lp.panel().get_length()
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
            for (child, chunk) in active_panels.iter().zip(chunks) {
                match child {
                    UserSplits::Panel(panel_i) => match app.get_panel(*panel_i) {
                        None => (), // error
                        Some(lp) => {
                            let is_active = *panel_i == app.active_panel();
                            if is_active {
                                let (x, y) = lp.panel().get_cursor(&chunk);
                                frame.set_cursor(x, y);
                            }

                            // if selecting, display id on top right side
                            let title = match app.selecting_panel() {
                                true => Spans::from(vec![
                                    Span::styled(
                                        format!(" {} ", lp.panel().get_id()),
                                        Style::default()
                                            .fg(Color::Green)
                                            .bg(Color::White)
                                            .add_modifier(Modifier::BOLD),
                                    ),
                                    Span::styled(
                                        lp.panel().get_title(),
                                        Style::default().fg(Color::White),
                                    ),
                                ]),
                                false => Spans::from(vec![Span::styled(
                                    lp.panel().get_title(),
                                    Style::default().fg(Color::White),
                                )]),
                            };

                            let block = Block::default()
                                .title(title)
                                .borders(Borders::ALL)
                                .border_style(Style::default().fg(match is_active {
                                    true => Color::Green,
                                    false => Color::White,
                                }));

                            lp.panel().make_widget(app, frame, chunk, is_active, block);
                        }
                    },
                    UserSplits::Split(split_index) => {
                        match app.get_split(*split_index) {
                            None => (), // error
                            Some(_) => render_split(*split_index, app, frame, chunk),
                        }
                    }
                }
            }
        }
    }
}
