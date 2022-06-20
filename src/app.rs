use std::collections::HashSet;

use crossterm::event::KeyCode;
use tui::layout::Direction;

use crate::autocomplete::{AutoCompleter, PanelAutoCompleter};
use crate::commands::ctrl_alt_key;
use crate::panels::{PanelFactory, NULL_PANEL_TYPE_ID};
use crate::{
    catch_all, ctrl_key, key, CommandDetails, Commands, InputPanel, Panel, PanelSplit, Panels,
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

pub enum StateChangeRequest {
    // String - prompt to display for input
    Input(String, Option<Box<dyn AutoCompleter>>),
    InputComplete(String),
    Message(Message),
}

impl StateChangeRequest {
    pub fn input_request(prompt: String) -> StateChangeRequest {
        StateChangeRequest::Input(prompt, None)
    }

    pub fn input_complete(text: String) -> StateChangeRequest {
        StateChangeRequest::InputComplete(text)
    }

    pub fn error<T: ToString>(message: T) -> StateChangeRequest {
        StateChangeRequest::Message(Message::error(message))
    }
}

const TOP_REQUESTOR_ID: usize = usize::MAX;

pub struct InputRequest {
    prompt: String,
    auto_completer: Option<Box<dyn AutoCompleter>>,
    requestor_id: usize,
}

impl InputRequest {
    pub fn prompt(&self) -> &String {
        &self.prompt
    }

    pub fn completer(&self) -> Option<&Box<dyn AutoCompleter>> {
        self.auto_completer.as_ref()
    }
}

pub struct LayoutPanel {
    split_index: usize,
    id: char,
    panel_index: usize,
}

impl LayoutPanel {
    fn new(split_index: usize, id: char, panel_index: usize) -> Self {
        Self {
            split_index,
            id,
            panel_index,
        }
    }

    pub fn panel_index(&self) -> usize {
        self.panel_index
    }

    pub fn id(&self) -> char {
        self.id
    }

    pub fn split(&self) -> usize {
        self.split_index
    }

    pub fn set_split(&mut self, split: usize) {
        self.split_index = split;
    }
}

pub struct AppState {
    panels: Vec<LayoutPanel>,
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
        AppState {
            panels: vec![],
            splits: vec![],
            active_panel: 0,
            selecting_panel: false,
            static_panels: vec![],
            messages: vec![],
            input_request: None,
            state: State::Normal,
        }
    }

    pub fn init(&mut self, panels: &mut Panels) {
        self.reset(panels);
    }

    pub fn add_error<T: ToString>(&mut self, message: T) {
        self.messages.push(Message::error(message));
    }

    pub fn add_info<T: ToString>(&mut self, message: T) {
        self.messages.push(Message::info(message));
    }

    pub fn reset(&mut self, panels: &mut Panels) {
        self.splits = vec![PanelSplit::new(
            Direction::Vertical,
            vec![UserSplits::Panel(0), UserSplits::Panel(1), UserSplits::Panel(2)],
        )];

        let mut input = PanelFactory::input();
        let mut edit = PanelFactory::edit();
        let mut messages = PanelFactory::messages();

        input.init(self);
        edit.init(self);
        messages.init(self);

        let input_index = panels.push(input);
        let edit_index = panels.push(edit);
        let messages_index = panels.push(messages);

        self.panels = vec![
            LayoutPanel::new(0, PROMPT_PANEL_ID, input_index),
            LayoutPanel::new(0, 'a', edit_index),
            LayoutPanel::new(0, 'b', messages_index),
        ];
        self.active_panel = 1;
        self.selecting_panel = false;
        self.static_panels = vec![PROMPT_PANEL_ID];
        self.state = State::Normal;
        self.input_request = None;
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

    pub fn get_active_panel(&mut self) -> Option<&LayoutPanel> {
        self.get_panel(self.active_panel)
    }

    pub fn get_active_panel_mut(&mut self) -> Option<&mut LayoutPanel> {
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

    pub fn get_panel(&self, index: usize) -> Option<&LayoutPanel> {
        self.panels.get(index)
    }

    pub fn get_panel_mut(&mut self, index: usize) -> Option<&mut LayoutPanel> {
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

    pub fn input_request(&self) -> Option<&InputRequest> {
        self.input_request.as_ref()
    }

    pub fn first_available_id(&mut self) -> char {
        let mut current = HashSet::new();

        for lp in self.panels.iter() {
            current.insert(lp.id);
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
        // let mut changes = vec![];
        // for lp in self.panels.iter_mut().filter(|lp| lp.visible()) {
        //     changes.extend(lp.panel.update());
        // }
        //
        // self.handle_changes(changes);
    }

    pub fn handle_changes(&mut self, changes: Vec<StateChangeRequest>, panels: &mut Panels) {
        let active_panel_id = match self.get_active_panel() {
            Some(lp) => lp.id,
            None => {
                self.messages
                    .push(Message::error("No active panel for change request."));
                return;
            }
        };

        for change in changes {
            let additional_changes = match change {
                StateChangeRequest::Input(prompt, completer) => {
                    // only one input request at a time, override existing
                    if self.static_panels.contains(&active_panel_id) {
                        self.messages
                            .push(Message::error("Input panel cannot make input request."));
                        return;
                    }

                    self.input_request = Some(InputRequest {
                        prompt: prompt.clone(),
                        auto_completer: completer,
                        requestor_id: self.active_panel,
                    });

                    self.active_panel = 0;

                    match self.get_panel(0) {
                        Some(lp) => match panels.get_mut(lp.panel_index) {
                            Some(panel) => panel.show(),
                            None => unimplemented!(),
                        },
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

                    self.input_request = None;

                    let changes = if index == TOP_REQUESTOR_ID {
                        match self.state {
                            State::WaitingPanelType(for_panel) => {
                                match self.get_panel(for_panel) {
                                    None => unimplemented!(),
                                    Some(lp) => match panels.get_mut(lp.panel_index) {
                                        Some(panel) => match PanelFactory::panel(input.as_str()) {
                                            Some(new) => *panel = new,
                                            None => unimplemented!(),
                                        },
                                        None => unimplemented!(),
                                    },
                                }

                                self.active_panel = for_panel;
                                self.state = State::Normal;
                            }
                            State::Normal => unimplemented!(),
                        }

                        vec![]
                    } else {
                        let changes = match self.get_panel(index) {
                            Some(lp) => match panels.get_mut(lp.panel_index) {
                                Some(panel) => panel.receive_input(input),
                                None => unimplemented!(),
                            },
                            None => {
                                self.messages
                                    .push(Message::error("Requesting panel doesn't exist."));
                                return;
                            }
                        };

                        self.active_panel = index;

                        changes
                    };

                    match self.get_panel(0) {
                        Some(lp) => match panels.get_mut(lp.panel_index) {
                            Some(panel) => panel.hide(),
                            None => unimplemented!(),
                        },
                        None => unimplemented!(),
                    }

                    changes
                }
                StateChangeRequest::Message(message) => {
                    self.messages.push(message);
                    vec![]
                }
            };

            self.handle_changes(additional_changes, panels);
        }
    }

    //
    // Command Actions
    //

    pub fn start_selecting_panel(&mut self, _code: KeyCode, _panels: &mut Panels) {
        self.selecting_panel = true;
    }

    pub fn select_panel(&mut self, code: KeyCode, _panels: &mut Panels) {
        self.selecting_panel = false;
        match code {
            KeyCode::Char(c) => match self.panels.iter().enumerate().find(|(_, lp)| lp.id == c) {
                None => {
                    self.messages
                        .push(Message::info(format!("No panel with ID '{}'", c)));
                }
                Some((index, _)) => {
                    self.set_active_panel(index);
                    if self.input_request.is_some() {
                        self.input_request = None;
                        self.messages.push(Message::info(
                            "Canceled input request due to panel selection.",
                        ))
                    }
                }
            },
            _ => {
                self.messages.push(Message::info(
                    "Invalid key for panel id. Options are letters a-z, lower or capital.",
                ));
            }
        }
    }

    pub fn split_current_panel_horizontal(&mut self, _code: KeyCode, panels: &mut Panels) {
        // opposite direction, because visual like will be vertical for horizontal layout
        self.split(Direction::Vertical, panels)
    }

    pub fn split_current_panel_vertical(&mut self, _code: KeyCode, panels: &mut Panels) {
        // opposite direction, because visual like will be horizontal for vertical layout
        self.split(Direction::Horizontal, panels)
    }

    pub fn add_panel_to_active_split(&mut self, _code: KeyCode, panels: &mut Panels) {
        let active_split = match self.get_active_panel() {
            Some(lp) => lp.split_index,
            None => {
                self.add_error("No active panel. Setting to be last panel.");
                self.active_panel = 1;
                return;
            }
        };

        let new_panel_index = self.add_panel(active_split, panels);

        match self.splits.get_mut(active_split) {
            Some(s) => s.panels.push(UserSplits::Panel(new_panel_index)),
            None => {
                self.add_error("Active panel's split not found. Resetting state.");
                self.reset(panels);
                return;
            }
        }
    }

    pub(crate) fn add_panel(&mut self, split: usize, panels: &mut Panels) -> usize {
        let new_id = self.first_available_id();
        let new_index = panels.push(PanelFactory::edit());

        self.panels.push(LayoutPanel::new(split, new_id, new_index));

        new_index
    }

    pub fn delete_active_panel(&mut self, _code: KeyCode, panels: &mut Panels) {
        let (next_active_panel, active_split, active_panel_id, active_panel_index) =
            match (self.next_panel_index(panels), self.get_active_panel()) {
                (Err(e), None) | (Err(e), _) => {
                    self.reset(panels);
                    self.messages.push(e);
                    return;
                }
                (_, None) => {
                    self.active_panel = 1;
                    self.messages
                        .push(Message::error("No active panel. Setting to be last panel."));
                    return;
                }
                (Ok(next), Some(lp)) => (next, lp.split_index, lp.id, lp.panel_index),
            };

        if self.static_panels().contains(&active_panel_id) {
            self.messages
                .push(Message::info(format!("Cannot delete static panel.")));
            return;
        }

        // find active's index in split
        let local_current_panel = self.active_panel();

        let remove_split = match self.splits.get_mut(active_split) {
            None => {
                self.messages.push(Message::error(
                    "Active panels split doesn't exist. Resetting state.",
                ));
                self.reset(panels);
                return;
            }
            Some(split) => {
                let index = match split.panels.iter().enumerate().find(|(_, s)| match s {
                    UserSplits::Panel(index) => *index == local_current_panel,
                    UserSplits::Split(..) => false,
                }) {
                    Some(i) => i.0,
                    None => {
                        self.messages.push(Message::error(
                            "Active panel's split doesn't contain active panel. Resetting state.",
                        ));
                        self.reset(panels);
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
                self.reset(panels);
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
                    self.reset(panels);
                    return;
                }
            }
        }

        // verified that it exists from first check getting active panel
        // self.panels.remove(local_current_panel);
        panels.remove(active_panel_index);

        let active_count = self
            .panels
            .iter()
            .filter(|lp| {
                panels
                    .get(lp.panel_index)
                    .map(|panel| panel.panel_type() != NULL_PANEL_TYPE_ID)
                    .unwrap_or(false)
            })
            .count();

        // if this is last panel besides static panels
        // we will replace it
        if active_count <= self.static_panels.len() {
            // use last split that we have for new panel's split
            let last = self.splits_len() - 1;
            let index = self.add_panel(last, panels);
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
                    self.reset(panels);
                    return;
                }
            }

            self.active_panel = index;
        } else {
            self.active_panel = next_active_panel;
        }
    }

    pub fn activate_next_panel(&mut self, _code: KeyCode, panels: &mut Panels) {
        self.resolve_panel_change(self.next_panel_index(panels));
    }

    pub fn activate_previous_panel(&mut self, _code: KeyCode, panels: &mut Panels) {
        self.resolve_panel_change(self.previous_panel_index(panels));
    }

    pub fn change_active_panel_type(&mut self, _code: KeyCode, panels: &mut Panels) {
        self.state = State::WaitingPanelType(self.active_panel);
        self.active_panel = 0;
        self.input_request = Some(InputRequest {
            prompt: "Panel Type".to_string(),
            requestor_id: TOP_REQUESTOR_ID,
            auto_completer: Some(Box::new(PanelAutoCompleter::new())),
        });
        match self.get_panel(0) {
            Some(lp) => match panels.get_mut(lp.panel_index) {
                Some(panel) => panel.show(),
                None => unimplemented!(),
            },
            None => unimplemented!(),
        }
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

    fn next_panel_index(&self, panels: &Panels) -> Result<usize, Message> {
        self.active_panel_index(panels, |index, order| {
            if index + 1 >= order.len() {
                0
            } else {
                index + 1
            }
        })
    }

    fn previous_panel_index(&self, panels: &Panels) -> Result<usize, Message> {
        self.active_panel_index(panels, |index, order| {
            if index == 0 {
                order.len() - 1
            } else {
                index - 1
            }
        })
    }

    fn active_panel_index<F: FnOnce(usize, &Vec<usize>) -> usize>(
        &self,
        panels: &Panels,
        f: F,
    ) -> Result<usize, Message> {
        let order = self.build_order(panels)?;
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

    fn build_order(&self, panels: &Panels) -> Result<Vec<usize>, Message> {
        let mut order = vec![];
        self.push_panels(0, &mut order, panels)?;
        Ok(order)
    }

    fn push_panels(
        &self,
        split: usize,
        order: &mut Vec<usize>,
        panels: &Panels,
    ) -> Result<(), Message> {
        match self.splits.get(split) {
            None => return Err(Message::error("Child split not found in splits.")),
            Some(split) => {
                for child in split.panels.iter() {
                    match child {
                        UserSplits::Panel(panel_index) => match self.panels.get(*panel_index) {
                            Some(lp) => match panels.get(lp.panel_index) {
                                Some(panel) => match panel.panel_type() == NULL_PANEL_TYPE_ID {
                                    true => (),
                                    false => order.push(*panel_index),
                                },
                                None => unimplemented!(),
                            },
                            None => return Err(Message::error("Child panel not found in panels.")),
                        },
                        UserSplits::Split(split_index) => {
                            self.push_panels(*split_index, order, panels)?
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

type GlobalAction = fn(&mut AppState, KeyCode, &mut Panels);

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

    commands.insert(|b| {
        b.node(ctrl_key('p')).node(key('t')).action(
            CommandDetails::change_panel_type(),
            AppState::change_active_panel_type,
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

    use crate::app::{InputRequest, LayoutPanel, Message, MessageChannel, State, TOP_REQUESTOR_ID};
    use crate::panels::{PanelFactory, NULL_PANEL_TYPE_ID};
    use crate::{AppState, Panels, UserSplits};

    fn assert_is_default(app: &AppState) {
        assert_eq!(app.panels.len(), 3, "Panels not set");
        assert_eq!(app.splits.len(), 1, "Splits not set");
        assert_eq!(app.active_panel, 1, "Active panel not set");
        assert_eq!(app.selecting_panel, false, "Selecting panel not set");
        assert_eq!(app.static_panels, vec!['$'], "Static panels not set");
        assert_eq!(app.state, State::Normal);
        assert!(app.input_request.is_none());
    }

    #[test]
    fn set_default() {
        let mut panels = Panels::new();
        let mut app = AppState::new();
        app.init(&mut panels);
        app.split_current_panel_horizontal(KeyCode::Null, &mut panels);
        app.split_current_panel_horizontal(KeyCode::Null, &mut panels);
        app.split_current_panel_horizontal(KeyCode::Null, &mut panels);
        app.input_request = Some(InputRequest {
            prompt: "Prompt".to_string(),
            requestor_id: TOP_REQUESTOR_ID,
            auto_completer: None,
        });
        app.state = State::WaitingPanelType(1);
        app.set_selecting_panel(true);

        app.reset(&mut panels);

        assert_is_default(&app);
    }

    #[test]
    fn select_panel() {
        let mut panels = Panels::new();
        let mut app = AppState::new();
        app.init(&mut panels);
        app.selecting_panel = true;
        app.add_panel_to_active_split(KeyCode::Null, &mut panels);

        app.select_panel(KeyCode::Char('b'), &mut panels);

        assert_eq!(app.active_panel, 2);
        assert!(!app.selecting_panel);
    }

    #[test]
    fn select_panel_invalid_code() {
        let mut panels = Panels::new();
        let mut app = AppState::new();
        app.init(&mut panels);
        app.selecting_panel = true;
        app.add_panel_to_active_split(KeyCode::Null, &mut panels);

        app.select_panel(KeyCode::Enter, &mut panels);

        assert_eq!(app.active_panel, 1);
        assert_eq!(app.messages[0].channel, MessageChannel::INFO);
    }

    #[test]
    fn select_panel_invalid_id() {
        let mut panels = Panels::new();
        let mut app = AppState::new();
        app.init(&mut panels);
        app.selecting_panel = true;
        app.add_panel_to_active_split(KeyCode::Null, &mut panels);

        app.select_panel(KeyCode::Char('z'), &mut panels);

        assert_eq!(app.active_panel, 1);
        assert_eq!(app.messages[0].channel, MessageChannel::INFO);
    }

    #[test]
    fn select_panel_cancels_input_request() {
        let mut panels = Panels::new();
        let mut app = AppState::new();
        app.init(&mut panels);
        app.selecting_panel = true;
        app.add_panel_to_active_split(KeyCode::Null, &mut panels);
        app.input_request = Some(InputRequest {
            prompt: "Test".to_string(),
            requestor_id: TOP_REQUESTOR_ID,
            auto_completer: None,
        });

        app.select_panel(KeyCode::Char('b'), &mut panels);

        assert_eq!(app.messages[0].channel, MessageChannel::INFO);
        assert!(app.input_request.is_none());
    }

    #[test]
    fn add_panel_to_active_split() {
        let mut panels = Panels::new();
        let mut app = AppState::new();
        app.init(&mut panels);

        app.add_panel_to_active_split(KeyCode::Null, &mut panels);

        assert_eq!(app.panels.len(), 4);
        assert_eq!(app.splits.len(), 1);

        assert_eq!(
            app.splits[0].panels,
            vec![
                UserSplits::Panel(0),
                UserSplits::Panel(1),
                UserSplits::Panel(2),
                UserSplits::Panel(3)
            ]
        );

        assert_eq!(app.panels[1].split_index, 0);
        assert_eq!(app.panels[2].split_index, 0);

        assert_eq!(app.panels[2].id, 'b')
    }

    #[test]
    fn add_panel_to_active_split_no_active_panel() {
        let mut panels = Panels::new();
        let mut app = AppState::new();
        app.init(&mut panels);
        app.active_panel = 100;

        app.add_panel_to_active_split(KeyCode::Null, &mut panels);

        assert_eq!(app.messages[0].channel, MessageChannel::ERROR)
    }

    #[test]
    fn add_panel_to_active_split_no_active_split() {
        let mut panels = Panels::new();
        let mut app = AppState::new();
        app.init(&mut panels);
        app.panels
            .push(LayoutPanel::new(10, 'b', panels.push(PanelFactory::edit())));
        app.active_panel = 3;

        app.add_panel_to_active_split(KeyCode::Null, &mut panels);

        assert_eq!(app.messages[0].channel, MessageChannel::ERROR)
    }

    #[test]
    fn split_panel() {
        let mut panels = Panels::new();
        let mut app = AppState::new();
        app.init(&mut panels);

        app.split_current_panel_horizontal(KeyCode::Null, &mut panels);

        assert_eq!(app.panels.len(), 4);
        assert_eq!(app.splits.len(), 2);

        assert_eq!(
            app.splits[1].panels,
            vec![UserSplits::Panel(1), UserSplits::Panel(3)]
        );

        assert_eq!(app.panels[3].split_index, 1);
    }

    #[test]
    fn split_panel_not_in_split_logs_message() {
        let mut panels = Panels::new();
        let mut app = AppState::new();
        app.init(&mut panels);
        app.splits[0].panels.remove(1);

        app.split_current_panel_horizontal(KeyCode::Null, &mut panels);

        assert_is_default(&app);
        assert_eq!(app.messages[0].channel, MessageChannel::ERROR)
    }

    #[test]
    fn split_no_active_panel_logs_message() {
        let mut panels = Panels::new();
        let mut app = AppState::new();
        app.init(&mut panels);
        app.set_active_panel(100);

        app.split_current_panel_horizontal(KeyCode::Null, &mut panels);

        assert_eq!(app.active_panel, 1);
        assert_eq!(app.messages[0].channel, MessageChannel::ERROR)
    }

    #[test]
    fn split_active_panel_split_non_existent_logs_message() {
        let mut panels = Panels::new();
        let mut app = AppState::new();
        app.init(&mut panels);
        app.panels
            .push(LayoutPanel::new(10, 'b', panels.push(PanelFactory::edit())));
        app.active_panel = 3;

        app.split_current_panel_horizontal(KeyCode::Null, &mut panels);

        assert_is_default(&app);
        assert_eq!(app.messages[0].channel, MessageChannel::ERROR)
    }

    #[test]
    fn prompt_panel_doesnt_split() {
        let mut panels = Panels::new();
        let mut app = AppState::new();
        app.init(&mut panels);

        app.set_active_panel(0);
        app.split_current_panel_horizontal(KeyCode::Null, &mut panels);

        assert_eq!(app.panels.len(), 3);
        assert_eq!(app.splits.len(), 1);
        assert!(app
            .messages
            .contains(&Message::info("Cannot split static panel")));
    }

    #[test]
    fn prompt_panel_doesnt_delete() {
        let mut panels = Panels::new();
        let mut app = AppState::new();
        app.init(&mut panels);

        app.set_active_panel(0);
        app.delete_active_panel(KeyCode::Null, &mut panels);

        assert_eq!(app.panels.len(), 3);
        assert_eq!(app.splits.len(), 1);

        assert!(app
            .messages
            .contains(&Message::info("Cannot delete static panel.")))
    }

    #[test]
    fn delete_active_panel() {
        let mut panels = Panels::new();
        let mut app = AppState::new();
        app.init(&mut panels);
        let next_panel_index = app.panels.len();

        app.split_current_panel_horizontal(KeyCode::Null, &mut panels);
        app.set_active_panel(next_panel_index);

        app.delete_active_panel(KeyCode::Null, &mut panels);

        assert_eq!(app.active_panel, 2);
        assert_eq!(app.panels.len(), 4);
        assert_eq!(app.splits.len(), 2);

        assert_eq!(panels.get(3).unwrap().panel_type(), NULL_PANEL_TYPE_ID);
    }

    #[test]
    fn delete_active_panel_replaces_if_only_one_left() {
        let mut panels = Panels::new();
        let mut app = AppState::new();
        app.init(&mut panels);

        app.delete_active_panel(KeyCode::Null, &mut panels);

        match app.get_active_panel() {
            Some(lp) => assert_ne!(
                panels.get(lp.panel_index).unwrap().panel_type(),
                NULL_PANEL_TYPE_ID
            ),
            None => panic!("No active panel"),
        }

        assert_eq!(panels.len(), 3);
        assert_eq!(app.splits.len(), 1);
    }

    #[test]
    fn delete_active_panel_deletes_empty_split() {
        let mut panels = Panels::new();
        let mut app = AppState::new();
        app.init(&mut panels);

        let second = app.panels.len();
        app.split_current_panel_horizontal(KeyCode::Null, &mut panels);
        app.set_active_panel(second);

        app.delete_active_panel(KeyCode::Null, &mut panels);

        app.set_active_panel(second);

        app.delete_active_panel(KeyCode::Null, &mut panels);

        assert_eq!(app.panels.len(), 3);
        assert_eq!(app.splits.len(), 1);
    }

    #[test]
    fn delete_invalid_active_panel_logs_message() {
        let mut panels = Panels::new();
        let mut app = AppState::new();
        app.init(&mut panels);
        app.active_panel = 100;

        app.delete_active_panel(KeyCode::Null, &mut panels);

        assert_eq!(app.active_panel, 1);
        assert_eq!(app.messages[0].channel, MessageChannel::ERROR)
    }

    #[test]
    fn delete_panel_with_invalid_split_logs_message() {
        let mut panels = Panels::new();
        let mut app = AppState::new();
        app.init(&mut panels);
        app.panels
            .push(LayoutPanel::new(10, 'b', panels.push(PanelFactory::edit())));
        app.active_panel = 3;

        app.delete_active_panel(KeyCode::Null, &mut panels);

        assert_is_default(&app);
        assert_eq!(app.messages[0].channel, MessageChannel::ERROR)
    }

    #[test]
    fn delete_panel_split_doesnt_have_panel_logs_message() {
        let mut panels = Panels::new();
        let mut app = AppState::new();
        app.init(&mut panels);
        app.splits[0].panels.remove(1);

        app.delete_active_panel(KeyCode::Null, &mut panels);

        assert_is_default(&app);
        assert_eq!(app.messages[0].channel, MessageChannel::ERROR)
    }

    #[test]
    fn delete_empty_split_not_present_in_parent_logs_message() {
        let mut panels = Panels::new();
        let mut app = AppState::new();
        app.init(&mut panels);

        let second = app.panels.len();
        app.split_current_panel_horizontal(KeyCode::Null, &mut panels);
        app.set_active_panel(second);

        let third = app.panels.len();
        app.split_current_panel_horizontal(KeyCode::Null, &mut panels);
        app.set_active_panel(third);

        app.splits[1].panels.remove(1);

        app.delete_active_panel(KeyCode::Null, &mut panels);
        app.set_active_panel(second);
        app.delete_active_panel(KeyCode::Null, &mut panels);

        assert_is_default(&app);
        assert_eq!(app.messages[0].channel, MessageChannel::ERROR)
    }

    #[test]
    fn activate_next_panel() {
        // 0 and 1
        let mut panels = Panels::new();
        let mut app = AppState::new();
        app.init(&mut panels);
        // 2
        app.split_current_panel_vertical(KeyCode::Null, &mut panels);
        // 3
        app.split_current_panel_horizontal(KeyCode::Null, &mut panels);
        // 4
        app.add_panel_to_active_split(KeyCode::Null, &mut panels);
        // 5
        app.add_panel_to_active_split(KeyCode::Null, &mut panels);
        app.set_active_panel(2);
        // 6
        app.split_current_panel_horizontal(KeyCode::Null, &mut panels);
        // 7
        app.add_panel_to_active_split(KeyCode::Null, &mut panels);
        // 8
        app.add_panel_to_active_split(KeyCode::Null, &mut panels);

        app.set_active_panel(7);

        app.activate_next_panel(KeyCode::Null, &mut panels);

        assert_eq!(app.active_panel(), 8)
    }

    #[test]
    fn activate_next_panel_skip_inactive() {
        // 0, 1 and 2
        let mut panels = Panels::new();
        let mut app = AppState::new();
        app.init(&mut panels);
        // 3
        app.split_current_panel_vertical(KeyCode::Null, &mut panels);
        // 4
        app.add_panel_to_active_split(KeyCode::Null, &mut panels);
        // 5
        app.add_panel_to_active_split(KeyCode::Null, &mut panels);
        app.set_active_panel(2);
        // 6
        app.split_current_panel_vertical(KeyCode::Null, &mut panels);
        // 7
        app.add_panel_to_active_split(KeyCode::Null, &mut panels);
        // 8
        app.add_panel_to_active_split(KeyCode::Null, &mut panels);

        app.set_active_panel(6);

        panels.remove(7);

        app.activate_next_panel(KeyCode::Null, &mut panels);

        assert_eq!(app.active_panel(), 8)
    }

    #[test]
    fn activate_next_panel_no_active_panel() {
        // 0 and 1
        let mut panels = Panels::new();
        let mut app = AppState::new();
        app.init(&mut panels);
        // 2
        app.split_current_panel_vertical(KeyCode::Null, &mut panels);
        // 3
        app.split_current_panel_horizontal(KeyCode::Null, &mut panels);
        // 4
        app.add_panel_to_active_split(KeyCode::Null, &mut panels);
        // 5
        app.add_panel_to_active_split(KeyCode::Null, &mut panels);
        app.set_active_panel(2);
        // 6
        app.split_current_panel_horizontal(KeyCode::Null, &mut panels);
        // 7
        app.add_panel_to_active_split(KeyCode::Null, &mut panels);
        // 8
        app.add_panel_to_active_split(KeyCode::Null, &mut panels);

        app.set_active_panel(10);

        app.activate_next_panel(KeyCode::Null, &mut panels);

        assert_eq!(app.active_panel, 1);
        assert_eq!(app.messages[0].channel, MessageChannel::ERROR)
    }

    #[test]
    fn activate_previous_panel() {
        // 0, 1 and 2
        let mut panels = Panels::new();
        let mut app = AppState::new();
        app.init(&mut panels);
        // 3
        app.split_current_panel_vertical(KeyCode::Null, &mut panels);
        // 4
        app.add_panel_to_active_split(KeyCode::Null, &mut panels);
        // 5
        app.add_panel_to_active_split(KeyCode::Null, &mut panels);
        app.set_active_panel(2);
        // 6
        app.split_current_panel_vertical(KeyCode::Null, &mut panels);
        // 7
        app.add_panel_to_active_split(KeyCode::Null, &mut panels);
        // 8
        app.add_panel_to_active_split(KeyCode::Null, &mut panels);

        app.set_active_panel(6);

        panels.remove(7);

        app.activate_previous_panel(KeyCode::Null, &mut panels);

        assert_eq!(app.active_panel(), 2)
    }

    #[test]
    fn activate_previous_panel_no_active_panel() {
        // 0 and 1
        let mut panels = Panels::new();
        let mut app = AppState::new();
        app.init(&mut panels);
        // 2
        app.split_current_panel_vertical(KeyCode::Null, &mut panels);
        // 3
        app.split_current_panel_horizontal(KeyCode::Null, &mut panels);
        // 4
        app.add_panel_to_active_split(KeyCode::Null, &mut panels);
        // 5
        app.add_panel_to_active_split(KeyCode::Null, &mut panels);
        app.set_active_panel(2);
        // 6
        app.split_current_panel_horizontal(KeyCode::Null, &mut panels);
        // 7
        app.add_panel_to_active_split(KeyCode::Null, &mut panels);
        // 8
        app.add_panel_to_active_split(KeyCode::Null, &mut panels);

        app.set_active_panel(10);

        app.activate_previous_panel(KeyCode::Null, &mut panels);

        assert_eq!(app.active_panel, 1);
        assert_eq!(app.messages[0].channel, MessageChannel::ERROR)
    }

    #[test]
    fn new_panel_after_delete_uses_inactive_slot() {
        let mut panels = Panels::new();
        let mut app = AppState::new();
        app.init(&mut panels);

        app.add_panel_to_active_split(KeyCode::Null, &mut panels);
        app.add_panel_to_active_split(KeyCode::Null, &mut panels);

        app.delete_active_panel(KeyCode::Null, &mut panels);

        assert_eq!(panels.get(1).unwrap().panel_type(), NULL_PANEL_TYPE_ID);

        app.add_panel_to_active_split(KeyCode::Null, &mut panels);

        assert_ne!(panels.get(1).unwrap().panel_type(), NULL_PANEL_TYPE_ID);
    }

    #[test]
    fn split_panel_after_delete_uses_inactive_slot() {
        let mut panels = Panels::new();
        let mut app = AppState::new();
        app.init(&mut panels);

        app.add_panel_to_active_split(KeyCode::Null, &mut panels);
        app.add_panel_to_active_split(KeyCode::Null, &mut panels);

        app.delete_active_panel(KeyCode::Null, &mut panels);

        assert_eq!(panels.get(1).unwrap().panel_type(), NULL_PANEL_TYPE_ID);

        app.split_current_panel_horizontal(KeyCode::Null, &mut panels);

        assert_ne!(panels.get(1).unwrap().panel_type(), NULL_PANEL_TYPE_ID);
    }
}

#[cfg(test)]
mod state_changes {
    use crossterm::event::KeyCode;
    use tui::text::Span;

    use crate::app::StateChangeRequest::InputComplete;
    use crate::app::{
        InputRequest, LayoutPanel, MessageChannel, State, StateChangeRequest, TOP_REQUESTOR_ID,
    };
    use crate::panels::MESSAGE_PANEL_TYPE_ID;
    use crate::{AppState, Panel, Panels};

    #[allow(dead_code)]
    struct TestPanel {
        expected_input: String,
        actual_input: String,
    }

    impl Panel for TestPanel {
        fn panel_type(&self) -> &str {
            "Test"
        }

        fn make_title(&self, _app: &AppState) -> Vec<Span> {
            vec![Span::raw(&self.actual_input)]
        }

        fn receive_input(&mut self, input: String) -> Vec<StateChangeRequest> {
            self.actual_input = input;
            vec![]
        }
    }

    #[test]
    fn input_request() {
        let mut panels = Panels::new();
        let mut app = AppState::new();
        app.init(&mut panels);

        app.handle_changes(
            vec![StateChangeRequest::input_request("Test Input".to_string())],
            &mut panels,
        );

        let request = app.input_request().unwrap();
        assert_eq!(request.prompt, "Test Input".to_string());
        assert_eq!(request.requestor_id, 1);
        assert!(request.auto_completer.is_none());
        assert_eq!(app.active_panel, 0);
    }

    #[test]
    fn input_request_no_active_panel() {
        let mut panels = Panels::new();
        let mut app = AppState::new();
        app.init(&mut panels);
        app.active_panel = 100;

        app.handle_changes(
            vec![StateChangeRequest::input_request("Test Input".to_string())],
            &mut panels,
        );

        assert_eq!(app.messages[0].channel, MessageChannel::ERROR)
    }

    #[test]
    fn input_request_input_panel_is_active() {
        let mut panels = Panels::new();
        let mut app = AppState::new();
        app.init(&mut panels);
        app.active_panel = 0;

        app.handle_changes(
            vec![StateChangeRequest::input_request("Test Input".to_string())],
            &mut panels,
        );

        assert_eq!(app.messages[0].channel, MessageChannel::ERROR)
    }

    #[test]
    fn input_complete() {
        let mut panels = Panels::new();
        let mut app = AppState::new();
        app.init(&mut panels);
        app.input_request = Some(InputRequest {
            prompt: "Test Input".to_string(),
            requestor_id: 1,
            auto_completer: None,
        });
        app.active_panel = 0;

        let panel = TestPanel {
            expected_input: "Test Input".to_string(),
            actual_input: "".to_string(),
        };

        app.panels[1] = LayoutPanel::new(0, 'a', panels.push(Box::new(panel)));

        app.handle_changes(
            vec![StateChangeRequest::input_complete("Test Input".to_string())],
            &mut panels,
        );

        assert!(app.input_request.is_none());
        assert_eq!(app.active_panel, 1);
    }

    #[test]
    fn input_complete_no_request() {
        let mut panels = Panels::new();
        let mut app = AppState::new();
        app.init(&mut panels);

        let panel = TestPanel {
            expected_input: "Test Input".to_string(),
            actual_input: "".to_string(),
        };

        app.panels[1] = LayoutPanel::new(0, 'a', panels.push(Box::new(panel)));

        app.handle_changes(
            vec![StateChangeRequest::input_complete("Test Input".to_string())],
            &mut panels,
        );

        assert!(app.input_request.is_none());
        assert_eq!(app.messages[0].channel, MessageChannel::ERROR)
    }

    #[test]
    fn input_complete_requestor_doesnt_exist() {
        let mut panels = Panels::new();
        let mut app = AppState::new();
        app.init(&mut panels);
        app.input_request = Some(InputRequest {
            prompt: "Test Input".to_string(),
            requestor_id: 10,
            auto_completer: None,
        });

        let panel = TestPanel {
            expected_input: "Test Input".to_string(),
            actual_input: "".to_string(),
        };

        app.panels[1] = LayoutPanel::new(0, 'a', panels.push(Box::new(panel)));

        app.handle_changes(
            vec![StateChangeRequest::input_complete("Test Input".to_string())],
            &mut panels,
        );

        assert!(app.input_request.is_none());
        assert_eq!(app.messages[0].channel, MessageChannel::ERROR)
    }

    #[test]
    fn error_message() {
        let mut panels = Panels::new();
        let mut app = AppState::new();
        app.init(&mut panels);

        app.handle_changes(
            vec![StateChangeRequest::error("Test Input".to_string())],
            &mut panels,
        );

        assert_eq!(app.messages[0].channel, MessageChannel::ERROR)
    }

    #[test]
    fn change_panel_type() {
        let mut panels = Panels::new();
        let mut app = AppState::new();
        app.init(&mut panels);

        app.change_active_panel_type(KeyCode::Null, &mut panels);

        assert_eq!(app.active_panel, 0);
        assert_eq!(app.state, State::WaitingPanelType(1));

        let request = app.input_request().unwrap();
        assert_eq!(request.prompt, "Panel Type".to_string());
        assert_eq!(request.requestor_id, TOP_REQUESTOR_ID);
        assert!(request.auto_completer.is_some());
    }

    #[test]
    fn change_panel_type_complete() {
        let mut panels = Panels::new();
        let mut app = AppState::new();
        app.init(&mut panels);
        app.active_panel = 0;
        app.state = State::WaitingPanelType(1);
        app.input_request = Some(InputRequest {
            prompt: "Panel Type".to_string(),
            requestor_id: TOP_REQUESTOR_ID,
            auto_completer: None,
        });

        app.handle_changes(
            vec![InputComplete(MESSAGE_PANEL_TYPE_ID.to_string())],
            &mut panels,
        );

        assert_ne!(app.get_panel(1).unwrap().id, '\0');
        assert_eq!(app.active_panel, 1);
        assert_eq!(app.state, State::Normal);
        assert!(app.input_request.is_none())
    }
}
