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
    let active_split = match app.get_active_panel() {
        None => {
            panic!("active panel not found")
        }
        Some((split_i, _active_panel)) => *split_i,
    };

    let new_split_index = app.splits_len();
    let new_id = app.first_available_id();
    let new_panel_index = app.panels_len();

    // create panel
    let mut p = TextEditPanel::new();
    p.set_id(new_id);
    app.push_panel((new_split_index, Box::new(p)));

    // set active panel's split to new split index
    match app.get_active_panel_mut() {
        None => unimplemented!(),
        Some((split, _)) => *split = new_split_index,
    }

    let new_panel_split = PanelSplit::new(
        direction,
        vec![
            UserSplits::Panel(app.active_panel()),
            UserSplits::Panel(new_panel_index),
        ],
    );

    // replace active panel within its split with new split
    let active_panel_index = app.active_panel();
    let new_split = match app.get_split_mut(active_split) {
        None => {
            panic!("split not found")
        }
        Some(split) => {
            // find child index for active panel
            let mut child_index = 0;
            for (i, child) in split.panels.iter().enumerate() {
                match child {
                    UserSplits::Split(_) => (),
                    UserSplits::Panel(addr) => {
                        if *addr == active_panel_index {
                            child_index = i;
                            break;
                        }
                    }
                }
            }

            split.panels[child_index] = UserSplits::Split(new_split_index);

            new_panel_split
        }
    };

    app.push_split(new_split);
}
