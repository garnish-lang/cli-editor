use crossterm::event::{KeyCode, KeyModifiers};

use crate::app::StateChangeRequest;
use crate::commands::{alt_catch_all, alt_key, code, shift_alt_key, shift_catch_all};
use crate::panels::{InputPanel, PanelTypeID, EDIT_PANEL_TYPE_ID, INPUT_PANEL_TYPE_ID};
use crate::{
    catch_all, ctrl_key, global_commands, AppState, CommandDetails, CommandKeyId, Commands, Panels,
    TextPanel,
};

type PanelCommand = fn(&mut TextPanel, KeyCode, &mut AppState) -> (bool, Vec<StateChangeRequest>);

type GlobalAction = fn(&mut AppState, KeyCode, &mut Panels);

pub const EDIT_COMMAND_INDEX: usize = 0;
pub const INPUT_COMMAND_INDEX: usize = 1;

pub struct Manager {
    state_commands: Commands<GlobalAction>,
    command_stack: Vec<usize>,
    commands: Vec<Commands<PanelCommand>>,
    progress: Vec<CommandKeyId>,
}

impl Default for Manager {
    fn default() -> Self {
        Manager {
            state_commands: global_commands().unwrap(),
            command_stack: vec![],
            commands: vec![
                make_edit_commands().unwrap(),
                make_input_commands().unwrap(),
                make_messages_commands().unwrap(),
            ],
            progress: vec![],
        }
    }
}

impl Manager {
    pub fn advance(&mut self, by: CommandKeyId, state: &mut AppState, panels: &mut Panels) {
        self.progress.push(by.clone());

        // state.add_info(format!("Checking stack {:?}", self.command_stack));

        let global_result = self.state_commands.get(&self.progress);
        let panel_result = self
            .command_stack
            .last()
            .and_then(|i| self.commands.get(*i))
            .and_then(|commands| commands.get(&self.progress));

        let fallthrough = match panel_result {
            None => true,
            Some((end, action)) => {
                // state.add_info(format!("Is end: {:?} | Have action: {:?}", end, action.is_some()));

                if end {
                    self.progress.clear();
                }
                match action {
                    None => true,
                    Some(action) => match panels.get_mut(state.active_panel()) {
                        None => true,
                        Some(panel) => {
                            let (handled, changes) = action(panel, by.code.clone(), state);
                            state.handle_changes(changes, panels);

                            !handled
                        }
                    }
                }
            }
        };

        if fallthrough {
            match global_result {
                None => (),
                Some((end, action)) => {
                    // state.add_info(format!("Not handled, checking global. Is end: {:?} | Have action: {:?}", end, action.is_some()));

                    if end {
                        self.progress.clear();
                    }
                    match action {
                        None => (),
                        Some(action) => action(state, by.code.clone(), panels),
                    }
                }
            }
        }
    }

    pub fn push_commands_for_panel(&mut self, type_id: PanelTypeID) {
        self.command_stack.push(match type_id {
            EDIT_PANEL_TYPE_ID => EDIT_COMMAND_INDEX,
            INPUT_PANEL_TYPE_ID => INPUT_COMMAND_INDEX,
            _ => return,
        });
    }

    pub fn replace_top_with_panel(&mut self, type_id: PanelTypeID) {
        match self.command_stack.pop() {
            // nothing to replace
            None => (),
            Some(_) => self.push_commands_for_panel(type_id),
        }
    }
}

//
// Command Defaults
//

pub fn make_edit_commands() -> Result<Commands<PanelCommand>, String> {
    let mut commands = Commands::<PanelCommand>::new();

    commands.insert(|b| {
        b.node(catch_all())
            .action(CommandDetails::empty(), TextPanel::handle_key_stroke)
    })?;

    commands.insert(|b| {
        b.node(shift_catch_all())
            .action(CommandDetails::empty(), TextPanel::handle_key_stroke)
    })?;

    commands.insert(|b| {
        b.node(ctrl_key('o'))
            .action(CommandDetails::open_file(), TextPanel::open_file)
    })?;

    commands.insert(|b| {
        b.node(ctrl_key('s'))
            .action(CommandDetails::empty(), TextPanel::save_buffer)
    })?;

    commands.insert(|b| {
        b.node(alt_key('i'))
            .action(CommandDetails::empty(), TextPanel::scroll_up_one)
    })?;

    commands.insert(|b| {
        b.node(alt_key('k'))
            .action(CommandDetails::empty(), TextPanel::scroll_down_one)
    })?;

    commands.insert(|b| {
        b.node(shift_alt_key('I'))
            .action(CommandDetails::empty(), TextPanel::scroll_up_ten)
    })?;

    commands.insert(|b| {
        b.node(shift_alt_key('K'))
            .action(CommandDetails::empty(), TextPanel::scroll_down_ten)
    })?;

    commands.insert(|b| {
        b.node(alt_key('w'))
            .action(CommandDetails::empty(), TextPanel::move_to_previous_line)
    })?;

    commands.insert(|b| {
        b.node(alt_key('a')).action(
            CommandDetails::empty(),
            TextPanel::move_to_previous_character,
        )
    })?;

    commands.insert(|b| {
        b.node(alt_key('s'))
            .action(CommandDetails::empty(), TextPanel::move_to_next_line)
    })?;

    commands.insert(|b| {
        b.node(alt_key('d'))
            .action(CommandDetails::empty(), TextPanel::move_to_next_character)
    })?;

    Ok(commands)
}

pub fn make_input_commands() -> Result<Commands<PanelCommand>, String> {
    let mut commands = Commands::<PanelCommand>::new();

    commands.insert(|b| {
        b.node(catch_all())
            .action(CommandDetails::empty(), InputPanel::handle_key_stroke)
    })?;

    commands.insert(|b| {
        b.node(shift_catch_all())
            .action(CommandDetails::empty(), InputPanel::handle_key_stroke)
    })?;

    commands.insert(|b| {
        b.node(alt_catch_all())
            .action(CommandDetails::empty(), InputPanel::fill_quick_select)
    })?;

    commands.insert(|b| {
        b.node(code(KeyCode::Tab)).action(
            CommandDetails::empty(),
            InputPanel::fill_current_quick_select,
        )
    })?;

    commands.insert(|b| {
        b.node(alt_key('='))
            .action(CommandDetails::empty(), InputPanel::next_quick_select)
    })?;

    commands.insert(|b| {
        b.node(alt_key('-'))
            .action(CommandDetails::empty(), InputPanel::previous_quick_select)
    })?;

    Ok(commands)
}

pub fn make_messages_commands() -> Result<Commands<PanelCommand>, String> {
    let mut commands = Commands::<PanelCommand>::new();

    Ok(commands)
}
