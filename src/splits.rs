use tui::layout::Direction;

use crate::AppState;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PanelSplit {
    pub direction: Direction,
    pub panels: Vec<UserSplits>,
}

impl PanelSplit {
    pub fn new(direction: Direction, panels: Vec<UserSplits>) -> Self {
        PanelSplit { direction, panels }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum UserSplits {
    Split(usize),
    Panel(usize),
}

impl AppState {
    pub fn split(&mut self, direction: Direction) {
        let new_split_index = self.splits_len();

        let (active_split, active_panel_id) = match self.get_active_panel_mut() {
            None => {
                self.add_error("No active panel. Setting to be last panel.");
                self.reset();
                return;
            }
            Some(lp) => {
                let r = (lp.split(), lp.panel().get_id());
                lp.set_split(new_split_index);
                r
            }
        };

        if self.static_panels().contains(&active_panel_id) {
            self.add_info("Cannot split static panel");
            return;
        }

        let new_panel_index = self.add_panel(new_split_index);

        let new_panel_split = PanelSplit::new(
            direction,
            vec![
                UserSplits::Panel(self.active_panel()),
                UserSplits::Panel(new_panel_index),
            ],
        );

        // replace active panel within its split with new split
        let active_panel_index = self.active_panel();
        let new_split = match self.get_split_mut(active_split) {
            None => {
                self.add_error("Active panel's split not found. Resetting state.");
                self.reset();
                return;
            }
            Some(split) => {
                // find child index for active panel
                let mut child_index = None;
                for (i, child) in split.panels.iter().enumerate() {
                    match child {
                        UserSplits::Split(_) => (),
                        UserSplits::Panel(addr) => {
                            if *addr == active_panel_index {
                                child_index = Some(i);
                                break;
                            }
                        }
                    }
                }

                match child_index {
                    Some(i) => split.panels[i] = UserSplits::Split(new_split_index),
                    None => {
                        self.add_error(
                            "Active panel not present in split. Setting to be last panel.",
                        );
                        self.reset();
                        return;
                    }
                }

                new_panel_split
            }
        };

        self.push_split(new_split);
    }
}
