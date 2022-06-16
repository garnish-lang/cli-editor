use crate::{split, Panel, PanelSplit, PromptPanel, TextEditPanel, UserSplits, Commands, key, ctrl_key, CommandDetails, catch_all};
use crossterm::event::KeyCode;
use std::collections::HashSet;
use tui::layout::Direction;

pub struct AppState {
    panels: Vec<(usize, Box<dyn Panel>)>,
    splits: Vec<PanelSplit>,
    active_panel: usize,
    selecting_panel: bool,
}

impl AppState {
    pub fn new() -> Self {
        let splits: Vec<PanelSplit> = vec![PanelSplit::new(
            Direction::Vertical,
            vec![UserSplits::Panel(0), UserSplits::Panel(1)],
        )];

        let mut text_panel = TextEditPanel::new();
        text_panel.set_id('a');

        let mut prompt_panel = PromptPanel::new();
        prompt_panel.set_id('$');

        let panels: Vec<(usize, Box<dyn Panel>)> =
            vec![(0, Box::new(prompt_panel)), (0, Box::new(text_panel))];

        let active_panel = 1;

        AppState {
            panels,
            splits,
            active_panel,
            selecting_panel: false,
        }
    }

    pub fn active_panel(&self) -> usize {
        self.active_panel
    }

    pub fn set_active_panel(&mut self, index: usize) {
        self.active_panel = index;
    }

    pub fn get_active_panel(&mut self) -> Option<&(usize, Box<dyn Panel>)> {
        self.get_panel(self.active_panel)
    }

    pub fn get_active_panel_mut(&mut self) -> Option<&mut (usize, Box<dyn Panel>)> {
        self.get_panel_mut(self.active_panel)
    }

    pub fn get_split(&self, index: usize) -> Option<&PanelSplit> {
        self.splits.get(index)
    }

    pub fn get_split_mut(&mut self, index: usize) -> Option<&mut PanelSplit> {
        self.splits.get_mut(index)
    }

    pub fn splits_len(&self) -> usize {
        self.splits.len()
    }

    pub fn push_split(&mut self, split: PanelSplit) {
        self.splits.push(split)
    }

    pub fn panels_len(&self) -> usize {
        self.panels.len()
    }

    pub fn get_panel(&self, index: usize) -> Option<&(usize, Box<dyn Panel>)> {
        self.panels.get(index)
    }

    pub fn get_panel_mut(&mut self, index: usize) -> Option<&mut (usize, Box<dyn Panel>)> {
        self.panels.get_mut(index)
    }

    pub fn push_panel(&mut self, panel: (usize, Box<dyn Panel>)) {
        self.panels.push(panel)
    }

    pub fn selecting_panel(&self) -> bool {
        self.selecting_panel
    }

    pub fn set_selecting_panel(&mut self, selecting: bool) {
        self.selecting_panel = selecting;
    }

    //
    // Command Actions
    //

    pub fn start_selecting_panel(&mut self, _code: KeyCode) {
        self.selecting_panel = true;
    }

    pub fn select_panel(&mut self, code: KeyCode) {
        self.selecting_panel = false;
        match code {
            KeyCode::Char(c) => {
                for (index, (_, panel)) in self.panels.iter().enumerate() {
                    if panel.get_id() == c {
                        self.set_active_panel(index);
                        break;
                    }
                }
            }
            _ => (), // soft error
        }
    }

    pub fn split_current_panel_horizontal(&mut self, _code: KeyCode) {
        split(self, Direction::Horizontal)
    }

    pub fn split_current_panel_vertical(&mut self, _code: KeyCode) {
        split(self, Direction::Vertical)
    }

    pub fn first_available_id(&mut self) -> char {
        let mut current = HashSet::new();

        for (_, panel) in self.panels.iter() {
            current.insert(panel.get_id());
        }

        let options = ('a'..'z').chain('A'..'Z');

        let mut id = '\0';
        for c in options {
            if !current.contains(&c) {
                id = c;
                break;
            }
        }

        id
    }
}

type GlobalAction = fn(&mut AppState, KeyCode);

pub fn global_commands() -> Commands<GlobalAction> {
    let mut commands = Commands::<GlobalAction>::new();

    commands
        .insert(|b| {
            b.node(ctrl_key('p')).node(key('h')).action(
                CommandDetails::split_horizontal(),
                AppState::split_current_panel_horizontal,
            )
        })
        .unwrap();

    commands
        .insert(|b| {
            b.node(ctrl_key('p')).node(key('v')).action(
                CommandDetails::split_vertical(),
                AppState::split_current_panel_vertical,
            )
        })
        .unwrap();

    commands
        .insert(|b| {
            b.node(ctrl_key('a').action(AppState::start_selecting_panel))
                .node(catch_all())
                .action(CommandDetails::select_panel(), AppState::select_panel)
        })
        .unwrap();

    commands
}