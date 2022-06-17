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
    let new_split_index = app.splits_len();
    let new_id = app.first_available_id();
    let new_panel_index = app.panels_len();

    let (active_split, active_panel_id) = match app.get_active_panel_mut() {
        None => {
            app.add_error("No active panel. Setting to be last panel.");
            app.reset();
            return;
        }
        Some((split_i, active_panel)) => {
            let r = (*split_i, active_panel.get_id());
            *split_i = new_split_index;
            r
        },
    };

    if app.static_panels().contains(&active_panel_id) {
        app.add_info("Cannot split static panel");
        return;
    }

    // create panel
    let mut p = TextEditPanel::new();
    p.set_id(new_id);
    app.push_panel((new_split_index, Box::new(p)));

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
            app.add_error("Active panel's split not found. Resetting state.");
            app.reset();
            return;
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
