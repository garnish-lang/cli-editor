use std::collections::HashSet;

use crossterm::event::KeyCode;
use tui::layout::Direction;

use crate::commands::ctrl_alt_key;
use crate::panels::NullPanel;
use crate::{
    catch_all, ctrl_key, key, CommandDetails, Commands, Panel, PanelSplit, PromptPanel,
    TextEditPanel, UserSplits,
};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum MessageChannel {
    ERROR,
    #[allow(dead_code)]
    WARNING,
    INFO,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Message {
    channel: MessageChannel,
    text: String,
}

impl Message {
    pub fn error<T: ToString>(text: T) -> Message {
        Message {
            channel: MessageChannel::ERROR,
            text: text.to_string(),
        }
    }

    pub fn info<T: ToString>(text: T) -> Message {
        Message {
            channel: MessageChannel::INFO,
            text: text.to_string(),
        }
    }
}

pub struct AppState {
    panels: Vec<(usize, Box<dyn Panel>)>,
    splits: Vec<PanelSplit>,
    active_panel: usize,
    selecting_panel: bool,
    static_panels: Vec<char>,
    messages: Vec<Message>,
}

const PROMPT_PANEL_ID: char = '$';

impl AppState {
    pub fn new() -> Self {
        let mut app = AppState {
            panels: vec![],
            splits: vec![],
            active_panel: 0,
            selecting_panel: false,
            static_panels: vec![],
            messages: vec![],
        };

        app.reset();

        app
    }

    pub fn add_error<T: ToString>(&mut self, message: T) {
        self.messages.push(Message::error(message));
    }

    pub fn add_info<T: ToString>(&mut self, message: T) {
        self.messages.push(Message::info(message));
    }

    pub fn reset(&mut self) {
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
        self.static_panels = vec![PROMPT_PANEL_ID]
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
        self.split(Direction::Horizontal)
    }

    pub fn split_current_panel_vertical(&mut self, _code: KeyCode) {
        self.split(Direction::Vertical)
    }

    pub fn add_panel_to_active_split(&mut self, _code: KeyCode) {
        let active_split = match self.get_active_panel() {
            Some((s, _)) => *s,
            None => {
                self.add_error("No active panel. Setting to be last panel.");
                self.active_panel = 1;
                return;
            }
        };

        let new_panel_index = self.add_panel(active_split);

        match self.splits.get_mut(active_split) {
            Some(s) => s.panels.push(UserSplits::Panel(new_panel_index)),
            None => {
                self.add_error("Active panel's split not found. Resetting state.");
                self.reset();
                return;
            }
        }
    }

    fn add_panel(&mut self, split: usize) -> usize {
        // find first inactive slot and replace value with new panel and given split
        match self
            .panels
            .iter_mut()
            .enumerate()
            .find(|(_, (_, panel))| !panel.get_active())
        {
            Some((i, v)) => {
                v.0 = split;
                v.1 = Box::new(TextEditPanel::new());
                i
            }
            // if there are no inactive panels, create new slot
            None => {
                self.panels.push((split, Box::new(TextEditPanel::new())));
                self.panels.len() - 1
            }
        }
    }

    pub fn delete_active_panel(&mut self, _code: KeyCode) {
        let (next_active_panel, active_split, active_panel_id) =
            match (self.next_panel_index(), self.get_active_panel()) {
                (Err(e), None) | (Err(e), _) => {
                    self.reset();
                    self.messages.push(e);
                    return;
                }
                (_, None) => {
                    self.active_panel = 1;
                    self.messages
                        .push(Message::error("No active panel. Setting to be last panel."));
                    return;
                }
                (Ok(next), Some((split_i, active_panel))) => {
                    (next, *split_i, active_panel.get_id())
                }
            };

        if self.static_panels().contains(&active_panel_id) {
            self.messages
                .push(Message::info(format!("Cannot delete static panel.")));
            return;
        }

        // find active's index in split
        let active_panel_index = self.active_panel();

        let remove_split = match self.splits.get_mut(active_split) {
            None => {
                self.messages.push(Message::error(
                    "Active panels split doesn't exist. Resetting state.",
                ));
                self.reset();
                return;
            }
            Some(split) => {
                let index = match split.panels.iter().enumerate().find(|(_, s)| match s {
                    UserSplits::Panel(index) => *index == active_panel_index,
                    UserSplits::Split(..) => false,
                }) {
                    Some(i) => i.0,
                    None => {
                        self.messages.push(Message::error(
                            "Active panel's split doesn't contain active panel. Resetting state.",
                        ));
                        self.reset();
                        return;
                    }
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
            'outer: for (i, s) in self.splits.iter().enumerate() {
                for (j, p) in s.panels.iter().enumerate() {
                    match p {
                        UserSplits::Panel(_) => (), // skip panels
                        UserSplits::Split(index) => {
                            if *index == active_split {
                                parent_index = i;
                                child_index = j;
                                break 'outer;
                            }
                        }
                    }
                }
            }

            if parent_index == 0 && child_index == 0 {
                self.messages.push(Message::error(
                    "Split not found in parent when removing due to being empty. Resetting state.",
                ));
                self.reset();
                return;
            }

            match self.get_split_mut(parent_index) {
                Some(p) => {
                    p.panels.remove(child_index);
                }
                None => {
                    // should be unreachable
                    // indexes used were gotten by enumerate
                    // so they should exist

                    self.messages.push(Message::error(
                        "Invalid split index after enumeration. Resetting state.",
                    ));
                    self.reset();
                    return;
                }
            }
        }

        // verified that it exists from first check getting active panel
        self.panels[active_panel_index] = (0, Box::new(NullPanel::new()));

        let active_count = self.panels.iter().filter(|(_, p)| p.get_active()).count();

        // if this is last panel besides static panels
        // we will replace it
        if active_count <= self.static_panels.len() {
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
                None => {
                    // should be unreachable
                    // getting here means splits is empty
                    // which should only be possible if we had removed the prompt panel
                    // causing the removal of top split
                    // this is caught above during the split removal

                    self.messages
                        .push(Message::error("No splits remaining. Resetting state."));
                    self.reset();
                    return;
                }
            }

            self.panels.push((last, Box::new(text_panel)));
            self.active_panel = index;
        } else {
            self.active_panel = next_active_panel;
        }
    }

    pub fn activate_next_panel(&mut self, _code: KeyCode) {
        self.resolve_panel_change(self.next_panel_index());
    }

    pub fn activate_previous_panel(&mut self, _code: KeyCode) {
        self.resolve_panel_change(self.previous_panel_index());
    }

    fn resolve_panel_change(&mut self, r: Result<usize, Message>) {
        match r {
            Ok(next) => self.active_panel = next,
            Err(e) => {
                self.active_panel = 1;
                self.messages.push(e);
            }
        }
    }

    fn next_panel_index(&self) -> Result<usize, Message> {
        self.active_panel_index(|index, order| {
            if index + 1 >= order.len() {
                0
            } else {
                index + 1
            }
        })
    }

    fn previous_panel_index(&self) -> Result<usize, Message> {
        self.active_panel_index(|index, order| {
            if index == 0 {
                order.len() - 1
            } else {
                index - 1
            }
        })
    }

    fn active_panel_index<F: FnOnce(usize, &Vec<usize>) -> usize>(
        &self,
        f: F,
    ) -> Result<usize, Message> {
        let order = self.build_order()?;
        let mut active_panel_index = None;
        for (i, panel_index) in order.iter().enumerate() {
            if *panel_index == self.active_panel {
                active_panel_index = Some(i);
            }
        }

        match active_panel_index {
            None => Err(Message::error("Active panel not found after ordering.")),
            Some(index) => Ok(order[f(index, &order)]),
        }
    }

    fn build_order(&self) -> Result<Vec<usize>, Message> {
        let mut nodes = vec![0];
        let mut order = vec![];

        while let Some(node) = nodes.pop() {
            match self.splits.get(node) {
                None => return Err(Message::error("Child split not found in splits.")),
                Some(split) => {
                    for panel in &split.panels {
                        match panel {
                            UserSplits::Panel(panel_index) => match self.panels.get(*panel_index) {
                                Some((_, panel)) => match panel.get_active() {
                                    true => order.push(*panel_index),
                                    false => (),
                                },
                                None => {
                                    return Err(Message::error("Child panel not found in panels."))
                                }
                            },
                            UserSplits::Split(split_index) => nodes.push(*split_index),
                        }
                    }
                }
            }
        }

        Ok(order)
    }
}

type GlobalAction = fn(&mut AppState, KeyCode);

pub fn global_commands() -> Result<Commands<GlobalAction>, String> {
    let mut commands = Commands::<GlobalAction>::new();

    //
    // Panel creation/deletion
    //

    commands.insert(|b| {
        b.node(ctrl_key('p')).node(key('h')).action(
            CommandDetails::split_horizontal(),
            AppState::split_current_panel_horizontal,
        )
    })?;

    commands.insert(|b| {
        b.node(ctrl_key('p')).node(key('v')).action(
            CommandDetails::split_vertical(),
            AppState::split_current_panel_vertical,
        )
    })?;

    commands.insert(|b| {
        b.node(ctrl_key('p')).node(key('n')).action(
            CommandDetails::add_panel(),
            AppState::add_panel_to_active_split,
        )
    })?;

    commands.insert(|b| {
        b.node(ctrl_key('p')).node(key('d')).action(
            CommandDetails::remove_panel(),
            AppState::delete_active_panel,
        )
    })?;

    //
    // Panel Navigation
    //
    commands.insert(|b| {
        b.node(ctrl_alt_key('l')).action(
            CommandDetails::activate_next_panel(),
            AppState::activate_next_panel,
        )
    })?;

    commands.insert(|b| {
        b.node(ctrl_alt_key('j')).action(
            CommandDetails::activate_previous_panel(),
            AppState::activate_previous_panel,
        )
    })?;

    //
    // Panel Selection
    //

    commands.insert(|b| {
        b.node(ctrl_key('a').action(AppState::start_selecting_panel))
            .node(catch_all())
            .action(CommandDetails::select_panel(), AppState::select_panel)
    })?;

    Ok(commands)
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyCode;

    use crate::app::{Message, MessageChannel};
    use crate::panels::NullPanel;
    use crate::{AppState, Panel, TextEditPanel, UserSplits};

    fn assert_is_default(app: &AppState) {
        assert_eq!(app.panels.len(), 2, "Panels not set");
        assert_eq!(app.splits.len(), 1, "Splits not set");
        assert_eq!(app.active_panel, 1, "Active panel not set");
        assert_eq!(app.selecting_panel, false, "Selecting panel not set");
        assert_eq!(app.static_panels, vec!['$'], "Static panels not set");
    }

    #[test]
    fn set_default() {
        let mut app = AppState::new();
        app.split_current_panel_horizontal(KeyCode::Null);
        app.split_current_panel_horizontal(KeyCode::Null);
        app.split_current_panel_horizontal(KeyCode::Null);
        app.set_selecting_panel(true);

        app.reset();

        assert_is_default(&app);
    }

    #[test]
    fn add_panel_to_active_split() {
        let mut app = AppState::new();

        app.add_panel_to_active_split(KeyCode::Null);

        assert_eq!(app.panels.len(), 3);
        assert_eq!(app.splits.len(), 1);

        assert_eq!(
            app.splits[0].panels,
            vec![
                UserSplits::Panel(0),
                UserSplits::Panel(1),
                UserSplits::Panel(2)
            ]
        );

        assert_eq!(app.panels[1].0, 0);
        assert_eq!(app.panels[2].0, 0);
    }

    #[test]
    fn add_panel_to_active_split_no_active_panel() {
        let mut app = AppState::new();
        app.active_panel = 100;

        app.add_panel_to_active_split(KeyCode::Null);

        assert_eq!(app.messages[0].channel, MessageChannel::ERROR)
    }

    #[test]
    fn add_panel_to_active_split_no_active_split() {
        let mut app = AppState::new();
        app.panels.push((10, Box::new(TextEditPanel::new())));
        app.active_panel = 2;

        app.add_panel_to_active_split(KeyCode::Null);

        assert_eq!(app.messages[0].channel, MessageChannel::ERROR)
    }

    #[test]
    fn split_panel() {
        let mut app = AppState::new();

        app.split_current_panel_horizontal(KeyCode::Null);

        assert_eq!(app.panels.len(), 3);
        assert_eq!(app.splits.len(), 2);

        assert_eq!(
            app.splits[1].panels,
            vec![UserSplits::Panel(1), UserSplits::Panel(2)]
        );

        assert_eq!(app.panels[1].0, 1);
        assert_eq!(app.panels[2].0, 1);
    }

    #[test]
    fn split_panel_not_in_split_logs_message() {
        let mut app = AppState::new();
        app.splits[0].panels.remove(1);

        app.split_current_panel_horizontal(KeyCode::Null);

        assert_is_default(&app);
        assert_eq!(app.messages[0].channel, MessageChannel::ERROR)
    }

    #[test]
    fn split_no_active_panel_logs_message() {
        let mut app = AppState::new();
        app.set_active_panel(100);

        app.split_current_panel_horizontal(KeyCode::Null);

        assert_eq!(app.active_panel, 1);
        assert_eq!(app.messages[0].channel, MessageChannel::ERROR)
    }

    #[test]
    fn split_active_panel_split_non_existent_logs_message() {
        let mut app = AppState::new();
        app.panels.push((10, Box::new(TextEditPanel::new())));
        app.active_panel = 2;

        app.split_current_panel_horizontal(KeyCode::Null);

        assert_is_default(&app);
        assert_eq!(app.messages[0].channel, MessageChannel::ERROR)
    }

    #[test]
    fn prompt_panel_doesnt_split() {
        let mut app = AppState::new();

        app.set_active_panel(0);
        app.split_current_panel_horizontal(KeyCode::Null);

        assert_eq!(app.panels.len(), 2);
        assert_eq!(app.splits.len(), 1);
        assert!(app
            .messages
            .contains(&Message::info("Cannot split static panel")));
    }

    #[test]
    fn prompt_panel_doesnt_delete() {
        let mut app = AppState::new();

        app.set_active_panel(0);
        app.delete_active_panel(KeyCode::Null);

        assert_eq!(app.panels.len(), 2);
        assert_eq!(app.splits.len(), 1);

        assert!(app
            .messages
            .contains(&Message::info("Cannot delete static panel.")))
    }

    #[test]
    fn delete_active_panel() {
        let mut app = AppState::new();
        let next_panel_index = app.panels_len();

        app.split_current_panel_horizontal(KeyCode::Null);
        app.set_active_panel(next_panel_index);

        app.delete_active_panel(KeyCode::Null);

        assert_eq!(app.active_panel, 0);
        assert_eq!(app.panels.len(), 3);
        assert_eq!(app.splits.len(), 2);

        assert!(!app.panels[2].1.get_active());
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
            None => panic!("No active panel"),
        }

        assert_eq!(app.panels.len(), 3);
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

        assert_eq!(app.panels.len(), 4);
        assert_eq!(app.splits.len(), 2);
    }

    #[test]
    fn delete_invalid_active_panel_logs_message() {
        let mut app = AppState::new();
        app.active_panel = 100;

        app.delete_active_panel(KeyCode::Null);

        assert_eq!(app.active_panel, 1);
        assert_eq!(app.messages[0].channel, MessageChannel::ERROR)
    }

    #[test]
    fn delete_panel_with_invalid_split_logs_message() {
        let mut app = AppState::new();
        app.panels.push((10, Box::new(TextEditPanel::new())));
        app.active_panel = 2;

        app.delete_active_panel(KeyCode::Null);

        assert_is_default(&app);
        assert_eq!(app.messages[0].channel, MessageChannel::ERROR)
    }

    #[test]
    fn delete_panel_split_doesnt_have_panel_logs_message() {
        let mut app = AppState::new();
        app.splits[0].panels.remove(1);

        app.delete_active_panel(KeyCode::Null);

        assert_is_default(&app);
        assert_eq!(app.messages[0].channel, MessageChannel::ERROR)
    }

    #[test]
    fn delete_empty_split_not_present_in_parent_logs_message() {
        let mut app = AppState::new();

        let second = app.panels_len();
        app.split_current_panel_horizontal(KeyCode::Null);
        app.set_active_panel(second);

        let third = app.panels_len();
        app.split_current_panel_horizontal(KeyCode::Null);
        app.set_active_panel(third);

        app.splits[1].panels.remove(1);

        app.delete_active_panel(KeyCode::Null);
        app.set_active_panel(second);
        app.delete_active_panel(KeyCode::Null);

        assert_is_default(&app);
        assert_eq!(app.messages[0].channel, MessageChannel::ERROR)
    }

    #[test]
    fn activate_next_panel() {
        // 0 and 1
        let mut app = AppState::new();
        // 2
        app.split_current_panel_vertical(KeyCode::Null);
        // 3
        app.split_current_panel_horizontal(KeyCode::Null);
        // 4
        app.add_panel_to_active_split(KeyCode::Null);
        // 5
        app.add_panel_to_active_split(KeyCode::Null);
        app.set_active_panel(2);
        // 6
        app.split_current_panel_horizontal(KeyCode::Null);
        // 7
        app.add_panel_to_active_split(KeyCode::Null);
        // 8
        app.add_panel_to_active_split(KeyCode::Null);

        app.set_active_panel(7);

        app.activate_next_panel(KeyCode::Null);

        assert_eq!(app.active_panel(), 8)
    }

    #[test]
    fn activate_next_panel_skip_inactive() {
        // 0 and 1
        let mut app = AppState::new();
        // 2
        app.split_current_panel_vertical(KeyCode::Null);
        // 3
        app.split_current_panel_horizontal(KeyCode::Null);
        // 4
        app.add_panel_to_active_split(KeyCode::Null);
        // 5
        app.add_panel_to_active_split(KeyCode::Null);
        app.set_active_panel(2);
        // 6
        app.split_current_panel_horizontal(KeyCode::Null);
        // 7
        app.add_panel_to_active_split(KeyCode::Null);
        // 8
        app.add_panel_to_active_split(KeyCode::Null);

        app.set_active_panel(6);

        app.panels[7] = (app.panels[7].0, Box::new(NullPanel::new()));

        app.activate_next_panel(KeyCode::Null);

        assert_eq!(app.active_panel(), 8)
    }

    #[test]
    fn activate_next_panel_no_active_panel() {
        // 0 and 1
        let mut app = AppState::new();
        // 2
        app.split_current_panel_vertical(KeyCode::Null);
        // 3
        app.split_current_panel_horizontal(KeyCode::Null);
        // 4
        app.add_panel_to_active_split(KeyCode::Null);
        // 5
        app.add_panel_to_active_split(KeyCode::Null);
        app.set_active_panel(2);
        // 6
        app.split_current_panel_horizontal(KeyCode::Null);
        // 7
        app.add_panel_to_active_split(KeyCode::Null);
        // 8
        app.add_panel_to_active_split(KeyCode::Null);

        app.set_active_panel(10);

        app.activate_next_panel(KeyCode::Null);

        assert_eq!(app.active_panel, 1);
        assert_eq!(app.messages[0].channel, MessageChannel::ERROR)
    }

    #[test]
    fn activate_previous_panel() {
        // 0 and 1
        let mut app = AppState::new();
        // 2
        app.split_current_panel_vertical(KeyCode::Null);
        // 3
        app.split_current_panel_horizontal(KeyCode::Null);
        // 4
        app.add_panel_to_active_split(KeyCode::Null);
        // 5
        app.add_panel_to_active_split(KeyCode::Null);
        app.set_active_panel(2);
        // 6
        app.split_current_panel_horizontal(KeyCode::Null);
        // 7
        app.add_panel_to_active_split(KeyCode::Null);
        // 8
        app.add_panel_to_active_split(KeyCode::Null);

        app.set_active_panel(7);

        app.activate_previous_panel(KeyCode::Null);

        assert_eq!(app.active_panel(), 6)
    }

    #[test]
    fn activate_previous_panel_no_active_panel() {
        // 0 and 1
        let mut app = AppState::new();
        // 2
        app.split_current_panel_vertical(KeyCode::Null);
        // 3
        app.split_current_panel_horizontal(KeyCode::Null);
        // 4
        app.add_panel_to_active_split(KeyCode::Null);
        // 5
        app.add_panel_to_active_split(KeyCode::Null);
        app.set_active_panel(2);
        // 6
        app.split_current_panel_horizontal(KeyCode::Null);
        // 7
        app.add_panel_to_active_split(KeyCode::Null);
        // 8
        app.add_panel_to_active_split(KeyCode::Null);

        app.set_active_panel(10);

        app.activate_previous_panel(KeyCode::Null);

        assert_eq!(app.active_panel, 1);
        assert_eq!(app.messages[0].channel, MessageChannel::ERROR)
    }

    #[test]
    fn new_panel_after_delete_uses_inactive_slot() {
        let mut app = AppState::new();

        app.add_panel_to_active_split(KeyCode::Null);
        app.add_panel_to_active_split(KeyCode::Null);

        app.delete_active_panel(KeyCode::Null);

        assert!(!app.panels[1].1.get_active());

        app.add_panel_to_active_split(KeyCode::Null);

        assert!(app.panels[1].1.get_active());
    }
}
