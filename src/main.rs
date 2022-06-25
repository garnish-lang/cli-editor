extern crate core;

use std::io;
use std::io::{Cursor, Stdout};

use crossterm::event::{read, DisableMouseCapture, Event, KeyCode};
use crossterm::execute;
use crossterm::style::Print;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use tui::backend::CrosstermBackend;
use tui::{Frame, Terminal};

use crate::app::{global_commands, AppState};
use crate::commands::{catch_all, ctrl_key, key, CommandDetails, CommandKeyId, Commands};
use crate::panels::{InputPanel, Panel, Panels, TextEditPanel};
use crate::render::{CURSOR_MAX, render_split};
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
    app_state.init(&mut panels);
    let mut global_commands = global_commands()?;

    loop {
        app_state.update();

        terminal
            .draw(|frame| render_split(0, &app_state, &panels, frame, frame.size()))
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

                let (end, action) = if global_commands.has_progress() {
                    global_commands.advance(CommandKeyId::new(event.code, event.modifiers))
                } else {
                    let (handled, changes) = match app_state.get_active_panel_mut() {
                        Some(lp) => match panels.get_mut(lp.panel_index()) {
                            Some(panel) => panel.receive_key(event, &mut app_state),
                            None => (false, vec![])
                        }
                        None => (false, vec![]), // error?
                    };

                    app_state.handle_changes(changes, &mut panels);

                    if handled {
                        (false, None)
                    } else {
                        global_commands.advance(CommandKeyId::new(event.code, event.modifiers))
                    }
                };

                match action {
                    Some(action) => action(&mut app_state, event.code, &mut panels),
                    None => (),
                };

                if end {
                    // reset
                    global_commands.reset();
                    app_state.set_selecting_panel(false);
                }
            }
            Event::Mouse(_event) => (), // println!("{:?}", event),
            Event::Resize(width, height) => execute!(
                terminal.backend_mut(),
                Print(format!("New size {}x{}", width, height))
            )
            .or_else(|err| Err(err.to_string()))?,
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
