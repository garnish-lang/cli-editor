use std::collections::HashSet;
use std::io;
use std::io::Stdout;

use crossterm::event::{read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode};
use crossterm::execute;
use crossterm::style::Print;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use tui::backend::CrosstermBackend;
use tui::layout::Direction;
use tui::{Frame, Terminal};

use crate::commands::{code, ctrl_key, key, CommandKeyId, Commands, CommandDetails, catch_all};
use crate::panels::{Panel, PromptPanel, TextEditPanel};
use crate::render::render_split;
use crate::splits::{split, PanelSplit, UserSplits};

mod commands;
mod panels;
mod render;
mod splits;

pub type EditorFrame<'a> = Frame<'a, CrosstermBackend<Stdout>>;

pub struct AppState {
    panels: Vec<(usize, Box<dyn Panel>)>,
    splits: Vec<PanelSplit>,
    active_panel: usize,
    selecting_panel: bool,
}

impl AppState {
    fn new() -> Self {
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

    pub fn set_selecting_panel(&mut self, _code: KeyCode) {
        self.selecting_panel = true;
    }

    pub fn select_panel(&mut self, code: KeyCode) {
        self.selecting_panel = false;
        match code {
            KeyCode::Char(c) => {
                for (index, (_, panel)) in self.panels.iter().enumerate() {
                    if panel.get_id() == c {
                        self.active_panel = index;
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

fn global_chords() -> Commands<GlobalAction> {
    let mut chords = Commands::<GlobalAction>::new();

    chords
        .insert(|b| {
            b.node(ctrl_key('s')).node(key('h')).action(
                CommandDetails::split_horizontal(),
                AppState::split_current_panel_horizontal,
            )
        })
        .unwrap();

    chords
        .insert(|b| {
            b.node(ctrl_key('s')).node(key('v')).action(
                CommandDetails::split_vertical(),
                AppState::split_current_panel_vertical,
            )
        })
        .unwrap();

    chords
        .insert(|b| {
            b.node(ctrl_key('a').action(AppState::set_selecting_panel))
                .node(catch_all())
                .action(CommandDetails::select_panel(), AppState::select_panel)
        })
        .unwrap();

    chords
}

fn main() -> Result<(), io::Error> {
    enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app_state = AppState::new();
    let mut global_commands = global_chords();

    loop {
        terminal.draw(|frame| render_split(0, &app_state, frame, frame.size()))?;

        match read()? {
            Event::Key(event) => {
                // Loop breaking doesn't work with current implementation
                if event.code == KeyCode::Esc {
                    break;
                }

                // allow active panel to receive first
                // unless global is in progress
                // if active panel doesn't handle event
                // then check global

                let (end, action) = if global_commands.has_progress() {
                    global_commands.advance(CommandKeyId::new(event.code, event.modifiers))
                } else {
                    let handled = match app_state.panels.get_mut(app_state.active_panel) {
                        Some((_, panel)) => panel.receive_key(event),
                        None => false, // error?
                    };

                    if handled {
                        global_commands.advance(CommandKeyId::new(event.code, event.modifiers))
                    } else {
                        (false, None)
                    }
                };

                match action {
                    Some(action) => action(&mut app_state, event.code),
                    None => (),
                };

                if end {
                    // reset
                    global_commands.reset();
                    app_state.selecting_panel = false;
                }
            }
            Event::Mouse(_event) => (), // println!("{:?}", event),
            Event::Resize(width, height) => execute!(
                terminal.backend_mut(),
                Print(format!("New size {}x{}", width, height))
            )?,
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
