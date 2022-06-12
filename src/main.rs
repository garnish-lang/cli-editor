use std::collections::HashSet;
use std::io;
use std::io::Stdout;

use crossterm::event::{
    read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers,
};
use crossterm::execute;
use crossterm::style::Print;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use tui::backend::CrosstermBackend;
use tui::layout::Direction;

use crate::chords::{Chords, KeyChord};
use crate::panels::{Panel, PromptPanel, TextEditPanel};
use crate::render::render_split;
use crate::splits::{PanelSplit, UserSplits};
use tui::{Frame, Terminal};

mod chords;
mod panels;
mod render;
mod splits;

pub type EditorFrame<'a> = Frame<'a, CrosstermBackend<Stdout>>;

fn first_available_id(panels: &Vec<(usize, Box<dyn Panel>)>) -> char {
    let mut current = HashSet::new();

    for (_, panel) in panels {
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

pub struct AppState {
    panels: Vec<(usize, Box<dyn Panel>)>,
    splits: Vec<PanelSplit>,
    active_panel: usize,
}

impl AppState {
    fn new() -> Result<Self, io::Error> {
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

        Ok(AppState {
            panels,
            splits,
            active_panel,
        })
    }
}

fn main() -> Result<(), io::Error> {
    enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app_state = AppState::new()?;
    let mut global_chords = Chords::global_chords();

    loop {
        terminal.draw(|frame| render_split(0, &app_state, frame, frame.size()))?;

        match read()? {
            Event::Key(event) => {
                // check if we're in a chord right now
                // if not, check if we're going to start a chord
                // then finally defer to non-chord commands
                let next_chord = match (&global_chords.current_chord, event.code) {
                    // soft error, just reset
                    // command should've been executed, before being set as current
                    (Some(KeyChord::Command(_)), _) => None,
                    (Some(KeyChord::Node(_, children)), code) => {
                        match children.get(&code) {
                            None => None, // end chord
                            Some(KeyChord::Command(f)) => {
                                // end of chord, execute function
                                f(&mut app_state);
                                None
                            }
                            Some(chord) => {
                                // set this chord as current chord
                                Some(chord.clone())
                            }
                        }
                    }
                    // not in chord, check other commands
                    (None, code) => {
                        // not in chord, but could start one
                        if event.modifiers.contains(KeyModifiers::CONTROL) {
                            // CTRL means a global command including chords
                            // chords without CONTROL will be deferred to active panel
                            match global_chords.chord_map.get(&code) {
                                Some(chord) => Some(chord.clone()),
                                None => None,
                            }
                        } else {
                            // defer to active panel
                            match app_state.panels.get_mut(app_state.active_panel) {
                                Some((_, panel)) => {
                                    if !panel.receive_key(event) {
                                        match event.code {
                                            KeyCode::Esc => break,
                                            _ => (),
                                        }
                                    }

                                    None
                                }
                                None => None,
                            }
                        }
                    }
                };

                global_chords.current_chord = next_chord;
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
