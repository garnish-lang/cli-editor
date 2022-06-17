use crate::{split, Panel, PanelSplit, PromptPanel, TextEditPanel, UserSplits, Commands, key, ctrl_key, CommandDetails, catch_all};
use crossterm::event::KeyCode;
use std::collections::HashSet;
use tui::layout::Direction;

pub struct AppState {
    panels: Vec<(usize, Box<dyn Panel>)>,
    splits: Vec<PanelSplit>,
    active_panel: usize,
    selecting_panel: bool,
    static_panels: Vec<char>
}

const PROMPT_PANEL_ID: char = '$';

impl AppState {
    pub fn new() -> Self {
        let mut app = AppState {
            panels: vec![],
            splits: vec![],
            active_panel: 0,
            selecting_panel: false,
            static_panels: vec![]
        };

        app.reset();

        app
    }

    fn reset(&mut self) {
        self.splits = vec![PanelSplit::new(
            Direction::Vertical,
            vec![UserSplits::Panel(0), UserSplits::Panel(1)],
        )];

        let mut text_panel = TextEditPanel::new();
        text_panel.set_id('a');

        let mut prompt_panel = PromptPanel::new();
        prompt_panel.set_id(PROMPT_PANEL_ID);

        self.panels = vec![(0, Box::new(prompt_panel)), (0, Box::new(text_panel))];
        self.active_panel = 1;
        self.selecting_panel = false;
    }

    pub fn static_panels(&self) -> &Vec<char> {
        &self.static_panels
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

    pub fn delete_active_panel(&mut self, _code: KeyCode) {
        let (active_split, active_panel_id) = match self.get_active_panel() {
            None => {
                panic!("active panel not found")
            }
            Some((split_i, active_panel)) => (*split_i, active_panel.get_id()),
        };

        if self.static_panels().contains(&active_panel_id) {
            return;
        }

        // find active's index in split
        let active_panel_index = self.active_panel();

        let remove_split = match self.splits.get_mut(active_split) {
            None => unimplemented!(),
            Some(split) => {
                let index = match split.panels.iter().enumerate().find(|(_, s)| match s {
                    UserSplits::Panel(index) => *index == active_panel_index,
                    UserSplits::Split(..) => false,
                }) {
                    Some(i) => i.0,
                    None => return, //error
                };

                split.panels.remove(index);

                split.panels.is_empty()
            }
        };

        if remove_split {
            self.splits.remove(active_split);

            // should always get set
            // if they remain zero, it would remove static prompt panel
            // error below
            let mut parent_index = 0;
            let mut child_index = 0;
            'outer: for (i, s)in self.splits.iter().enumerate() {
                for (j, p) in s.panels.iter().enumerate() {
                    match p {
                        UserSplits::Panel(_) => (),
                        UserSplits::Split(index) => if *index == active_split {
                            parent_index = i;
                            child_index = j;
                            break 'outer;
                        }
                    }
                }
            }

            if parent_index == 0 && child_index == 0 {
                unimplemented!()
            }

            match self.get_split_mut(parent_index) {
                Some(p) => {
                    p.panels.remove(child_index);
                },
                None => unimplemented!(), // error
            }
        }

        self.panels.remove(active_panel_index);

        // if this is last panel besides static panels
        // we will replace it
        if self.panels.len() <= self.static_panels.len() {
            // get id before removal so its different
            // done in order to detect change
            let new_id = self.first_available_id();
            let index = self.panels_len();
            let mut text_panel = TextEditPanel::new();
            text_panel.set_id(new_id);

            // use last split that we have for new panel's split
            let last = self.splits_len() - 1;
            match self.get_split_mut(last) {
                Some(s) => s.panels.push(UserSplits::Panel(index)),
                None => unimplemented!("No split when replacing removed panel: {}", last)
            }

            self.panels.push((last, Box::new(text_panel)));
        }
    }
}

type GlobalAction = fn(&mut AppState, KeyCode);

pub fn global_commands() -> Result<Commands<GlobalAction>, String> {
    let mut commands = Commands::<GlobalAction>::new();

    commands
        .insert(|b| {
            b.node(ctrl_key('p')).node(key('h')).action(
                CommandDetails::split_horizontal(),
                AppState::split_current_panel_horizontal,
            )
        })?;

    commands
        .insert(|b| {
            b.node(ctrl_key('p')).node(key('v')).action(
                CommandDetails::split_vertical(),
                AppState::split_current_panel_vertical,
            )
        })?;

    commands
        .insert(|b| {
            b.node(ctrl_key('a').action(AppState::start_selecting_panel))
                .node(catch_all())
                .action(CommandDetails::select_panel(), AppState::select_panel)
        })?;

    Ok(commands)
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyCode;
    use crate::{AppState, Panel, TextEditPanel};

    #[test]
    fn set_default() {
        let mut app = AppState::new();
        app.split_current_panel_horizontal(KeyCode::Null);
        app.split_current_panel_horizontal(KeyCode::Null);
        app.split_current_panel_horizontal(KeyCode::Null);
        app.set_selecting_panel(true);

        app.reset();

        assert_eq!(app.panels.len(), 2);
        assert_eq!(app.splits.len(), 1);
        assert_eq!(app.active_panel, 1);
        assert_eq!(app.selecting_panel, false);
    }

    #[test]
    fn split_panel() {
        let mut app = AppState::new();

        app.split_current_panel_horizontal(KeyCode::Null);

        assert_eq!(app.panels.len(), 3);
        assert_eq!(app.splits.len(), 2);
    }

    #[test]
    fn prompt_panel_doesnt_split() {
        let mut app = AppState::new();

        app.set_active_panel(0);
        app.split_current_panel_horizontal(KeyCode::Null);

        assert_eq!(app.panels.len(), 2);
        assert_eq!(app.splits.len(), 1);
    }

    #[test]
    fn prompt_panel_doesnt_delete() {
        let mut app = AppState::new();

        app.set_active_panel(0);
        app.delete_active_panel(KeyCode::Null);

        assert_eq!(app.panels.len(), 2);
        assert_eq!(app.splits.len(), 1);
    }

    #[test]
    fn delete_active_panel() {
        let mut app = AppState::new();
        let next_panel_index = app.panels_len();

        app.split_current_panel_horizontal(KeyCode::Null);
        app.set_active_panel(next_panel_index);

        app.delete_active_panel(KeyCode::Null);

        assert_eq!(app.panels.len(), 2);
        assert_eq!(app.splits.len(), 2);
    }

    #[test]
    fn delete_active_panel_replaces_if_only_one_left() {
        let mut app = AppState::new();
        let mut panel = TextEditPanel::new();
        panel.set_title("Temp".to_string());
        panel.set_id('a');

        app.delete_active_panel(KeyCode::Null);

        match app.get_active_panel() {
            Some((_, panel)) => assert_eq!(panel.get_title().clone(), "Editor".to_string()),
            None => panic!("No active panel")
        }

        assert_eq!(app.panels.len(), 2);
        assert_eq!(app.splits.len(), 1);
    }

    #[test]
    fn delete_active_panel_deletes_empty_split() {
        let mut app = AppState::new();

        let second = app.panels_len();
        app.split_current_panel_horizontal(KeyCode::Null);
        app.set_active_panel(second);

        let third = app.panels_len();
        app.split_current_panel_horizontal(KeyCode::Null);
        app.set_active_panel(third);

        app.delete_active_panel(KeyCode::Null);

        app.set_active_panel(second);

        app.delete_active_panel(KeyCode::Null);

        assert_eq!(app.panels.len(), 2);
        assert_eq!(app.splits.len(), 2);
    }
}