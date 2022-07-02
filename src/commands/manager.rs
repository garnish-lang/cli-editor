use crossterm::event::{KeyCode, KeyModifiers};
use crate::{AppState, catch_all, CommandDetails, CommandKeyId, Commands, ctrl_key, global_commands, InputPanel, Panels, TextPanel};
use crate::app::StateChangeRequest;
use crate::commands::{alt_catch_all, alt_key, code, shift_alt_key, shift_catch_all};

type PanelCommand =
fn(&mut TextPanel, KeyCode, &mut AppState) -> (bool, Vec<StateChangeRequest>);

type GlobalAction = fn(&mut AppState, KeyCode, &mut Panels);

pub struct Manager {
    state_commands: Commands<GlobalAction>,
    command_stack: Vec<usize>,
    commands: Vec<Commands<PanelCommand>>,

    // all commands so there only stored once
    edit_commands: Commands<PanelCommand>,
    input_commands: Commands<PanelCommand>,
    // messages_commands: Commands<PanelCommand>,
}

pub struct CommandProgress {
    keys: Vec<CommandKeyId>,
    commands_index: Option<usize>,
}

impl CommandProgress {
    pub fn start() -> Self {
        Self {
            keys: vec![],
            commands_index: None
        }
    }
}

impl Default for Manager {
    fn default() -> Self {
        Manager {
            state_commands: global_commands().unwrap(),
            command_stack: vec![],
            commands: vec![],
            // commands
            edit_commands: make_edit_commands(),
            input_commands: make_input_commands().unwrap(),
            // messages_commands: make_edit_commands(),

        }
    }
}

impl Manager {
    pub fn advance(&self, progress: CommandProgress, by: CommandKeyId) -> CommandProgress {

        CommandProgress {
            keys: vec![progress.keys, vec![by]].concat(),
            commands_index: progress.commands_index,
        }
    }
}


//
// Command Defaults
//

pub fn make_edit_commands() -> Commands<PanelCommand> {
    let mut commands = Commands::<PanelCommand>::new();

    commands.insert(|b| {
        b.node(catch_all())
            .action(CommandDetails::empty(), TextPanel::handle_key_stroke)
    }).unwrap();

    commands.insert(|b| {
        b.node(shift_catch_all())
            .action(CommandDetails::empty(), TextPanel::handle_key_stroke)
    }).unwrap();

    commands.insert(|b| {
        b.node(ctrl_key('o'))
            .action(CommandDetails::open_file(), TextPanel::open_file)
    }).unwrap();

    commands.insert(|b| {
        b.node(ctrl_key('s'))
            .action(CommandDetails::empty(), TextPanel::save_buffer)
    }).unwrap();

    commands.insert(|b| {
        b.node(alt_key('i'))
            .action(CommandDetails::empty(), TextPanel::scroll_up_one)
    }).unwrap();

    commands.insert(|b| {
        b.node(alt_key('k'))
            .action(CommandDetails::empty(), TextPanel::scroll_down_one)
    }).unwrap();

    commands.insert(|b| {
        b.node(shift_alt_key('I'))
            .action(CommandDetails::empty(), TextPanel::scroll_up_ten)
    }).unwrap();

    commands.insert(|b| {
        b.node(shift_alt_key('K'))
            .action(CommandDetails::empty(), TextPanel::scroll_down_ten)
    }).unwrap();

    commands.insert(|b| {
        b.node(alt_key('w')).action(
            CommandDetails::empty(),
            TextPanel::move_to_previous_line,
        )
    }).unwrap();

    commands.insert(|b| {
        b.node(alt_key('a')).action(
            CommandDetails::empty(),
            TextPanel::move_to_previous_character,
        )
    }).unwrap();

    commands.insert(|b| {
        b.node(alt_key('s'))
            .action(CommandDetails::empty(), TextPanel::move_to_next_line)
    }).unwrap();

    commands.insert(|b| {
        b.node(alt_key('d')).action(
            CommandDetails::empty(),
            TextPanel::move_to_next_character,
        )
    }).unwrap();

    commands
}

pub fn make_input_commands() -> Result<Commands<PanelCommand>, String> {
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