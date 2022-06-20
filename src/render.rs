use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{Block, Borders};

use crate::{AppState, EditorFrame, Panels};
use crate::panels::NULL_PANEL_TYPE_ID;
use crate::splits::UserSplits;

pub fn render_split(split: usize, app: &AppState, panels: &Panels, frame: &mut EditorFrame, chunk: Rect) {
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
                        Some(lp) => match panels.get(lp.panel_index()) {
                            Some(panel) => panel.visible() && panel.panel_type() != NULL_PANEL_TYPE_ID,
                            None => false
                        }
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
                            Some(lp) => match panels.get(lp.panel_index()) {
                                Some(panel) => match panel.get_length(app) {
                                    0 => (0, 0),
                                    n => (1, n),
                                }
                                None => (0, 0)
                            }
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
                                Some(lp) => match panels.get(lp.panel_index()) {
                                    Some(panel) => {
                                        if panel.get_length(app) == 0 {
                                            part_size
                                        } else {
                                            panel.get_length(app)
                                        }
                                    }
                                    None => part_size
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
                        Some(lp) => match panels.get(lp.panel_index()) {
                            Some(panel) => {
                                let is_active = *panel_i == app.active_panel();
                                if is_active {
                                    let (x, y) = panel.get_cursor();
                                    frame.set_cursor(chunk.x + x, chunk.y + y);
                                }

                                let mut title = vec![];

                                if app.selecting_panel() {
                                    title.push(
                                        Span::styled(
                                            format!(" {} ", lp.id()),
                                            Style::default()
                                                .fg(Color::Green)
                                                .bg(Color::White)
                                                .add_modifier(Modifier::BOLD),
                                        )
                                    );
                                }

                                title.extend(panel.make_title(&app));

                                let block = Block::default()
                                    .title(Spans::from(title))
                                    .borders(Borders::ALL)
                                    .border_style(Style::default().fg(match is_active {
                                        true => Color::Green,
                                        false => Color::White,
                                    }));

                                panel.make_widget(app, frame, chunk, is_active, block);
                            }
                            None => (),
                        }
                    },
                    UserSplits::Split(split_index) => {
                        match app.get_split(*split_index) {
                            None => (), // error
                            Some(_) => render_split(*split_index, app, panels, frame, chunk),
                        }
                    }
                }
            }
        }
    }
}
