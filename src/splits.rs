use tui::layout::Direction;

use crate::panels::{Panel, TextEditPanel};
use crate::AppState;

pub struct PanelSplit {
    pub direction: Direction,
    pub panels: Vec<UserSplits>,
}

impl PanelSplit {
    pub fn new(direction: Direction, panels: Vec<UserSplits>) -> Self {
        PanelSplit { direction, panels }
    }
}

pub enum UserSplits {
    Split(usize),
    Panel(usize),
}

pub fn split(app: &mut AppState, direction: Direction) {
    match app.panels.get(app.active_panel) {
        None => {
            panic!("active panel not found")
        }
        Some((split_i, active_panel)) => {
            let split_i = *split_i;
            let active_panel_id = active_panel.get_id();
            let split_count = app.splits.len();
            let new_id = app.first_available_id();
            let new_split = match app.splits.get_mut(split_i) {
                None => {
                    panic!("split not found")
                }
                Some(split) => {
                    // create split
                    let new_split_index = split_count;

                    // create panel
                    let new_panel_index = app.panels.len();
                    let mut p = TextEditPanel::new();
                    p.set_id(new_id);
                    app.panels.push((new_split_index, Box::new(p)));

                    // update active panel split index
                    let mut child_index = 0;
                    for (i, (_, child)) in app.panels.iter().enumerate() {
                        if child.get_id() == active_panel_id {
                            child_index = i;
                            break;
                        }
                    }

                    app.panels[child_index].0 = new_split_index;

                    let new_panel_split = PanelSplit::new(
                        direction,
                        vec![
                            UserSplits::Panel(app.active_panel),
                            UserSplits::Panel(new_panel_index),
                        ],
                    );

                    // find child index for active panel
                    let mut child_index = 0;
                    for (i, child) in split.panels.iter().enumerate() {
                        match child {
                            UserSplits::Split(_) => (),
                            UserSplits::Panel(addr) => {
                                match app.panels.get(*addr) {
                                    None => (), // error?
                                    Some((_, p)) => {
                                        if p.get_id() == active_panel_id {
                                            child_index = i;
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    split.panels[child_index] = UserSplits::Split(new_split_index);

                    new_panel_split
                }
            };

            app.splits.push(new_split);
        }
    }
}
