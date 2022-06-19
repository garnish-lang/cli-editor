use std::collections::HashSet;

use crossterm::event::KeyCode;
use tui::layout::Direction;

use crate::commands::ctrl_alt_key;
use crate::panels::{NullPanel, PanelFactory};
use crate::{
    catch_all, ctrl_key, key, CommandDetails, Commands, InputPanel, Panel, PanelSplit,
    TextEditPanel, UserSplits,
};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum MessageChannel {
    ERROR,
    #[allow(dead_code)]
    WARNING,
    INFO,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
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

    pub fn channel(&self) -> MessageChannel {
        self.channel
    }

    pub fn text(&self) -> &String {
        &self.text
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
enum State {
    Normal,
    WaitingPanelType(usize),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum StateChangeRequest {
    // String - prompt to display for input
    Input(String),
    InputComplete(String),
    Message(Message),
}

impl StateChangeRequest {
    pub fn input_request(prompt: String) -> StateChangeRequest {
        StateChangeRequest::Input(prompt)
    }

    pub fn input_complete(text: String) -> StateChangeRequest {
        StateChangeRequest::InputComplete(text)
    }

    pub fn error<T: ToString>(message: T) -> StateChangeRequest {
        StateChangeRequest::Message(Message::error(message))
    }
}

const TOP_REQUESTOR_ID: usize = usize::MAX;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
struct InputRequest {
    prompt: String,
    requestor_id: usize,
}

pub struct AppState {
    panels: Vec<(usize, Box<dyn Panel>)>,
    splits: Vec<PanelSplit>,
    active_panel: usize,
    selecting_panel: bool,
    static_panels: Vec<char>,
    messages: Vec<Message>,
    input_request: Option<InputRequest>,
    state: State,
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
            input_request: None,
            state: State::Normal,
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

        let mut prompt_panel = InputPanel::new();
        prompt_panel.set_id(PROMPT_PANEL_ID);

        text_panel.init(self);
        prompt_panel.init(self);

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

    pub fn get_panel(&self, index: usize) -> Option<&(usize, Box<dyn Panel>)> {
        self.panels.get(index)
    }

    pub fn get_panel_mut(&mut self, index: usize) -> Option<&mut (usize, Box<dyn Panel>)> {
        self.panels.get_mut(index)
    }

    pub fn selecting_panel(&self) -> bool {
        self.selecting_panel
    }

    pub fn set_selecting_panel(&mut self, selecting: bool) {
        self.selecting_panel = selecting;
    }

    pub fn get_messages(&self) -> &Vec<Message> {
        &self.messages
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

    pub fn update(&mut self) {
        let mut changes = vec![];
        for (_, p) in self
            .panels
            .iter_mut()
            .filter(|(_, p)| p.visible() && p.get_active())
        {
            changes.extend(p.update());
        }

        self.handle_changes(changes);
    }

    pub fn handle_changes(&mut self, changes: Vec<StateChangeRequest>) {
        let active_panel_id = match self.get_active_panel() {
            Some((_, panel)) => panel.get_id(),
            None => {
                self.messages
                    .push(Message::error("No active panel for change request."));
                return;
            }
        };

        for change in changes {
            let additional_changes = match change {
                StateChangeRequest::Input(prompt) => {
                    // only one input request at a time, override existing
                    if self.static_panels.contains(&active_panel_id) {
                        self.messages
                            .push(Message::error("Input panel cannot make input request."));
                        return;
                    }

                    self.input_request = Some(InputRequest {
                        prompt: prompt.clone(),
                        requestor_id: self.active_panel,
                    });

                    self.active_panel = 0;

                    match self.get_panel_mut(0) {
                        Some((_, panel)) => {
                            panel.show();
                            panel.set_title(prompt.clone());
                        }
                        None => unimplemented!(),
                    }

                    vec![]
                }
                StateChangeRequest::InputComplete(input) => {
                    let index = match &self.input_request {
                        Some(request) => request.requestor_id,
                        None => {
                            self.messages
                                .push(Message::error("No active input request."));
                            return;
                        }
                    };

                    let changes = if index == TOP_REQUESTOR_ID {
                        match self.state {
                            State::WaitingPanelType(for_panel) => {
                                match self.get_panel_mut(for_panel) {
                                    None => unimplemented!(),
                                    Some((_, panel)) => {
                                        *panel = match PanelFactory::panel(input.as_str()) {
                                            None => unimplemented!(),
                                            Some(p) => p,
                                        }
                                    }
                                }

                                self.active_panel = for_panel;
                                self.state = State::Normal;
                                self.input_request = None;
                            }
                            State::Normal => unimplemented!()
                        }

                        vec![]
                    } else {
                        let changes = match self.get_panel_mut(index) {
                            Some((_, panel)) => panel.receive_input(input),
                            None => {
                                self.messages
                                    .push(Message::error("Requesting panel doesn't exist."));
                                return;
                            }
                        };

                        self.active_panel = index;

                        changes
                    };

                    match self.get_panel_mut(0) {
                        Some((_, panel)) => {
                            panel.hide();
                        }
                        None => unimplemented!(),
                    }

                    changes
                }
                StateChangeRequest::Message(message) => {
                    self.messages.push(message);
                    vec![]
                }
            };

            self.handle_changes(additional_changes);
        }
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
        // opposite direction, because visual like will be vertical for horizontal layout
        self.split(Direction::Vertical)
    }

    pub fn split_current_panel_vertical(&mut self, _code: KeyCode) {
        // opposite direction, because visual like will be horizontal for vertical layout
        self.split(Direction::Horizontal)
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

    pub(crate) fn add_panel(&mut self, split: usize) -> usize {
        let new_id = self.first_available_id();
        let mut new_panel = Box::new(TextEditPanel::new());
        new_panel.init(self);
        new_panel.set_id(new_id);
        // find first inactive slot and replace value with new panel and given split
        match self
            .panels
            .iter_mut()
            .enumerate()
            .find(|(_, (_, panel))| !panel.get_active())
        {
            Some((i, v)) => {
                v.0 = split;
                v.1 = new_panel;
                i
            }
            // if there are no inactive panels, create new slot
            None => {
                self.panels.push((split, new_panel));
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
            // use last split that we have for new panel's split
            let last = self.splits_len() - 1;
            let index = self.add_panel(last);
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

    pub fn change_active_panel_type(&mut self, _code: KeyCode) {
        self.state = State::WaitingPanelType(self.active_panel);
        self.active_panel = 0;
        self.input_request = Some(InputRequest {
            prompt: "Panel Type".to_string(),
            requestor_id: TOP_REQUESTOR_ID,
        });
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
        let mut order = vec![];
        self.push_panels(0, &mut order)?;
        Ok(order)
    }

    fn push_panels(&self, split: usize, order: &mut Vec<usize>) -> Result<(), Message> {
        match self.splits.get(split) {
            None => return Err(Message::error("Child split not found in splits.")),
            Some(split) => {
                for child in split.panels.iter() {
                    match child {
                        UserSplits::Panel(panel_index) => match self.panels.get(*panel_index) {
                            Some((_, panel)) => match panel.get_active() {
                                true => order.push(*panel_index),
                                false => (),
                            },
                            None => return Err(Message::error("Child panel not found in panels.")),
                        },
                        UserSplits::Split(split_index) => self.push_panels(*split_index, order)?,
                    }
                }
            }
        }

        Ok(())
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

    use crate::app::{InputRequest, Message, MessageChannel, State, TOP_REQUESTOR_ID};
    use crate::panels::{MESSAGE_PANEL_TYPE_ID, NullPanel};
    use crate::{AppState, Panel, TextEditPanel, UserSplits};
    use crate::app::StateChangeRequest::InputComplete;

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

        assert_eq!(app.panels[2].1.get_id(), 'b')
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
        let next_panel_index = app.panels.len();

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
            Some((_, panel)) => assert_eq!(panel.get_title().clone(), "Buffer".to_string()),
            None => panic!("No active panel"),
        }

        assert_eq!(app.panels.len(), 2);
        assert_eq!(app.splits.len(), 1);
    }

    #[test]
    fn delete_active_panel_deletes_empty_split() {
        let mut app = AppState::new();

        let second = app.panels.len();
        app.split_current_panel_horizontal(KeyCode::Null);
        app.set_active_panel(second);

        let third = app.panels.len();
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

        let second = app.panels.len();
        app.split_current_panel_horizontal(KeyCode::Null);
        app.set_active_panel(second);

        let third = app.panels.len();
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

    #[test]
    fn split_panel_after_delete_uses_inactive_slot() {
        let mut app = AppState::new();

        app.add_panel_to_active_split(KeyCode::Null);
        app.add_panel_to_active_split(KeyCode::Null);

        app.delete_active_panel(KeyCode::Null);

        assert!(!app.panels[1].1.get_active());

        app.split_current_panel_horizontal(KeyCode::Null);

        assert!(app.panels[1].1.get_active());
    }

    #[test]
    fn change_panel_type() {
        let mut app = AppState::new();

        app.change_active_panel_type(KeyCode::Null);

        assert_eq!(app.active_panel, 0);
        assert_eq!(app.state, State::WaitingPanelType(1));
        assert_eq!(app.input_request, Some(InputRequest {
            prompt: "Panel Type".to_string(),
            requestor_id: TOP_REQUESTOR_ID
        }))
    }

    #[test]
    fn change_panel_type_complete() {
        let mut app = AppState::new();
        app.active_panel = 0;
        app.state = State::WaitingPanelType(1);
        app.input_request = Some(InputRequest {
            prompt: "Panel Type".to_string(),
            requestor_id: TOP_REQUESTOR_ID
        });

        app.handle_changes(vec![
            InputComplete(MESSAGE_PANEL_TYPE_ID.to_string())
        ]);

        assert_eq!(app.active_panel, 1);
        assert_eq!(app.state, State::Normal);
        assert!(app.input_request.is_none())
    }
}

#[cfg(test)]
mod state_changes {
    use crate::app::{InputRequest, MessageChannel, StateChangeRequest};
    use crate::{AppState, Panel};

    #[allow(dead_code)]
    struct TestPanel {
        expected_input: String,
        actual_input: String,
    }

    impl Panel for TestPanel {
        fn type_id(&self) -> &str {
            "Test"
        }

        fn get_title(&self) -> &str {
            &self.actual_input
        }

        fn receive_input(&mut self, input: String) -> Vec<StateChangeRequest> {
            self.actual_input = input;
            vec![]
        }
    }

    #[test]
    fn input_request() {
        let mut state = AppState::new();

        state.handle_changes(vec![StateChangeRequest::input_request(
            "Test Input".to_string(),
        )]);

        assert_eq!(
            state.input_request,
            Some(InputRequest {
                prompt: "Test Input".to_string(),
                requestor_id: 1
            })
        );
        assert_eq!(state.active_panel, 0);
    }

    #[test]
    fn input_request_no_active_panel() {
        let mut state = AppState::new();
        state.active_panel = 100;

        state.handle_changes(vec![StateChangeRequest::input_request(
            "Test Input".to_string(),
        )]);

        assert_eq!(state.messages[0].channel, MessageChannel::ERROR)
    }

    #[test]
    fn input_request_input_panel_is_active() {
        let mut state = AppState::new();
        state.active_panel = 0;

        state.handle_changes(vec![StateChangeRequest::input_request(
            "Test Input".to_string(),
        )]);

        assert_eq!(state.messages[0].channel, MessageChannel::ERROR)
    }

    #[test]
    fn input_complete() {
        let mut state = AppState::new();
        state.input_request = Some(InputRequest {
            prompt: "Test Input".to_string(),
            requestor_id: 1,
        });
        state.active_panel = 0;

        let mut panel = TestPanel {
            expected_input: "Test Input".to_string(),
            actual_input: "".to_string(),
        };

        panel.set_id('a');

        state.panels[1] = (0, Box::new(panel));

        state.handle_changes(vec![StateChangeRequest::input_complete(
            "Test Input".to_string(),
        )]);

        assert_eq!(state.active_panel, 1);
        assert_eq!(state.panels[1].1.get_title(), "Test Input".to_string());
    }

    #[test]
    fn input_complete_no_request() {
        let mut state = AppState::new();

        let mut panel = TestPanel {
            expected_input: "Test Input".to_string(),
            actual_input: "".to_string(),
        };

        panel.set_id('a');

        state.panels[1] = (0, Box::new(panel));

        state.handle_changes(vec![StateChangeRequest::input_complete(
            "Test Input".to_string(),
        )]);

        assert_eq!(state.messages[0].channel, MessageChannel::ERROR)
    }

    #[test]
    fn input_complete_requestor_doesnt_exist() {
        let mut state = AppState::new();
        state.input_request = Some(InputRequest {
            prompt: "Test Input".to_string(),
            requestor_id: 10,
        });

        let mut panel = TestPanel {
            expected_input: "Test Input".to_string(),
            actual_input: "".to_string(),
        };

        panel.set_id('a');

        state.panels[1] = (0, Box::new(panel));

        state.handle_changes(vec![StateChangeRequest::input_complete(
            "Test Input".to_string(),
        )]);

        assert_eq!(state.messages[0].channel, MessageChannel::ERROR)
    }

    #[test]
    fn error_message() {
        let mut state = AppState::new();

        state.handle_changes(vec![StateChangeRequest::error("Test Input".to_string())]);

        assert_eq!(state.messages[0].channel, MessageChannel::ERROR)
    }
}
