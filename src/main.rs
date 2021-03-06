extern crate core;

use std::io;
use std::io::Stdout;

use crossterm::event::{read, DisableMouseCapture, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use tui::backend::CrosstermBackend;
use tui::{Frame, Terminal};

use crate::app::{global_commands, AppState};
use crate::commands::{catch_all, ctrl_key, key, CommandDetails, CommandKeyId, Commands};
use crate::panels::{Panels, TextPanel};
use crate::render::{render_split, CURSOR_MAX};
use crate::splits::{PanelSplit, UserSplits};

mod app;
mod autocomplete;
mod commands;
mod panels;
mod render;
mod splits;

pub type EditorFrame<'a> = Frame<'a, CrosstermBackend<Stdout>>;

fn main() -> Result<(), String> {
    enable_raw_mode().or_else(|err| Err(err.to_string()))?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, DisableMouseCapture)
        .or_else(|err| Err(err.to_string()))?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).or_else(|err| Err(err.to_string()))?;

    let mut panels = Panels::new();
    let mut app_state = AppState::new();
    let mut commands = commands::Manager::default();
    app_state.init(&mut panels, &mut commands);

    // temp
    // to be replaced when saving layouts is implemented
    // don't want to change layout in state defaults everytime since it would continually break tests
    app_state.set_active_panel(2);
    app_state.split_current_panel_vertical(KeyCode::Null, &mut panels, &mut commands);
    match panels.get_mut(3) {
        None => app_state.add_error("Failed to update panel to commands."),
        Some(panel) => *panel = TextPanel::commands_panel(),
    }
    app_state.set_active_panel(1);

    loop {
        app_state.update();

        terminal
            .draw(|frame| render_split(0, &app_state, &commands, &panels, frame, frame.size()))
            .or_else(|err| Err(err.to_string()))?;

        // hide cursor if at max
        if terminal.get_cursor().unwrap_or_default() == CURSOR_MAX {
            terminal.hide_cursor().unwrap_or_default();
        } else {
            terminal.show_cursor().unwrap_or_default();
        }

        match read().or_else(|err| Err(err.to_string()))? {
            Event::Key(event) => {
                // Loop breaking doesn't work with current implementation
                if event.code == KeyCode::Esc {
                    break;
                }

                // allow active panel to receive first
                // unless global is in progress
                // if active panel doesn't handle event
                // then check global

                // Note for available controls as of crossterm "0.23"
                // CTRL with number keys and their symbols don't work
                // CTRL with 'i' and 'm' currently don't work
                // All ALT and SHIFT letters, numbers and symbols work
                //      except that shift symbols don't have the shift modifier
                //      even though the given char is correct
                // Shift not working with Backspace or Enter

                // app_state.add_info(format!("Received key: {:?} {:?}", event.code, event.modifiers));

                commands.advance(CommandKeyId::new(event.code, event.modifiers), &mut app_state, &mut panels);
            }
            Event::Mouse(_event) => (), // println!("{:?}", event),
            Event::Resize(_, _) => (),
        }
    }

    disable_raw_mode().or_else(|err| Err(err.to_string()))?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .or_else(|err| Err(err.to_string()))?;
    terminal.show_cursor().or_else(|err| Err(err.to_string()))?;

    Ok(())
}
