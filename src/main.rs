use std::io;
use std::io::Stdout;
use std::ops::Sub;

use crossterm::event::{
    read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
};
use crossterm::execute;
use crossterm::style::Print;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use tui::backend::CrosstermBackend;
use tui::buffer::Buffer;
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Style};
use tui::text::{Span, Text};
use tui::widgets::{Block, Borders, Paragraph, Widget, Wrap};
use tui::{Frame, Terminal};

trait Panel {
    fn make_widget(&self, frame: &mut EditorFrame, rect: Rect);
    fn get_cursor(&self, rect: &Rect) -> (u16, u16);
    fn receive_key(&mut self, event: KeyEvent) -> bool;
    fn set_active(&mut self);
}

// trait ChordReceiver {
//     fn receive(
//         &self,
//         current_panel: &mut Box<dyn Panel>,
//         app_state: &mut AppState,
//         code: KeyCode,
//     ) -> KeyInputState;
// }
//
// enum KeyInputState {
//     Normal,
// }

struct TextEditPanel {
    min_x: u16,
    min_y: u16,
    cursor_x: u16,
    cursor_y: u16,
    text: String,
}

impl TextEditPanel {
    fn new() -> Self {
        TextEditPanel {
            cursor_x: 1,
            cursor_y: 1,
            min_x: 1,
            min_y: 1,
            text: String::new(),
        }
    }
}

impl Panel for TextEditPanel {
    fn make_widget(&self, frame: &mut EditorFrame, rect: Rect) {
        let para_text = Text::from(self.text.clone());
        let para_block = Block::default().title("Block").borders(Borders::ALL);
        let para = Paragraph::new(para_text)
            .block(para_block)
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true });

        frame.render_widget(para, rect);
    }

    fn get_cursor(&self, rect: &Rect) -> (u16, u16) {
        (rect.x + self.cursor_x, rect.y + self.cursor_y)
    }

    fn receive_key(&mut self, event: KeyEvent) -> bool {
        match event.code {
            KeyCode::Backspace => {
                match self.text.pop() {
                    None => {
                        self.cursor_x = self.min_x;
                        self.cursor_y = self.min_y;
                    }
                    Some(c) => {
                        match c {
                            '\n' => {
                                self.cursor_y -= 1;
                                self.cursor_x = self.min_x;

                                // count from back until a newline is reached
                                for c in self.text.chars().rev() {
                                    if c == '\n' {
                                        break;
                                    }
                                    self.cursor_x += 1;
                                }
                            }
                            _ => {
                                self.cursor_x -= 1;
                            }
                        }
                    }
                }
            }
            KeyCode::Delete => {
                // ??
            }
            KeyCode::Enter => {
                self.text.push('\n');
                self.cursor_y += 1;
                self.cursor_x = 1;
            }
            KeyCode::Char(c) => {
                self.cursor_x += 1;
                self.text.push(c);
            }
            _ => return false,
        }

        true
    }

    fn set_active(&mut self) {
        todo!()
    }
}

struct PromptPanel {
    min_x: u16,
    min_y: u16,
    cursor_x: u16,
    cursor_y: u16,
    text: String,
}

impl PromptPanel {
    fn new() -> Self {
        PromptPanel {
            cursor_x: 1,
            cursor_y: 1,
            min_x: 1,
            min_y: 1,
            text: String::new(),
        }
    }
}

impl Panel for PromptPanel {
    fn make_widget(&self, frame: &mut EditorFrame, rect: Rect) {
        let para_text = Span::from(self.text.clone());
        let para_block = Block::default().title("Prompt").borders(Borders::ALL);
        let para = Paragraph::new(para_text)
            .block(para_block)
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true });

        frame.render_widget(para, rect);
    }

    fn get_cursor(&self, rect: &Rect) -> (u16, u16) {
        (rect.x + self.cursor_x, rect.y + self.cursor_y)
    }

    fn receive_key(&mut self, event: KeyEvent) -> bool {
        match event.code {
            KeyCode::Backspace => {
                match self.text.pop() {
                    None => {
                        self.cursor_x = self.min_x;
                        self.cursor_y = self.min_y;
                    }
                    Some(c) => {
                        match c {
                            '\n' => {
                                self.cursor_y -= 1;
                                self.cursor_x = self.min_x;

                                // count from back until a newline is reached
                                for c in self.text.chars().rev() {
                                    if c == '\n' {
                                        break;
                                    }
                                    self.cursor_x += 1;
                                }
                            }
                            _ => {
                                self.cursor_x -= 1;
                            }
                        }
                    }
                }
            }
            KeyCode::Delete => {
                // ??
            }
            KeyCode::Enter => {
                // perform action
            }
            KeyCode::Char(c) => {
                self.cursor_x += 1;
                self.text.push(c);
            }
            _ => return false,
        }

        true
    }

    fn set_active(&mut self) {
        todo!()
    }
}

enum UserSplits {
    Split(Direction, Vec<UserSplits>),
    Panel(usize),
}

const PROMPT_HEIGHT: u16 = 3;

type EditorFrame<'a> = Frame<'a, CrosstermBackend<Stdout>>;

fn main() -> Result<(), io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut splits: Vec<UserSplits> = vec![UserSplits::Split(
        Direction::Horizontal,
        vec![UserSplits::Panel(0)],
    )];
    let mut panels: Vec<(usize, Box<dyn Panel>)> = vec![(0, Box::new(TextEditPanel::new()))];
    let mut prompt_panel = PromptPanel::new();
    let mut active_panel = 0;

    loop {
        terminal.draw(|f| {
            let size = f.size();

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(PROMPT_HEIGHT),
                    Constraint::Length(size.height - PROMPT_HEIGHT),
                ])
                .split(size);

            let (prompt_chunk, user_chunk) = (chunks[0], chunks[1]);

            prompt_panel.make_widget(f, prompt_chunk);

            // first split should always be there
            match splits.get(0) {
                None => (), // error, recreate first split
                Some(split) => {
                    match split {
                        UserSplits::Panel(_) => (), // shouldn't happen?
                        UserSplits::Split(direction, children) => {
                            // calculate child width
                            let total = match direction {
                                Direction::Horizontal => user_chunk.width,
                                Direction::Vertical => user_chunk.height,
                            };

                            let lengths = match children.len() {
                                0 => vec![], // error?
                                1 => vec![Constraint::Length(total)],
                                n => {
                                    let part_size = total / n as u16;
                                    let mut remaining = total;
                                    let mut lengths = vec![];

                                    // loop for all but last item n - 1, range is exclusive on end
                                    // and set length to part size
                                    // subtract from remaining which will be last item's lengths
                                    for _ in 0..(n - 1) {
                                        remaining -= part_size;
                                        lengths.push(Constraint::Length(part_size));
                                    }

                                    lengths.push(Constraint::Length(remaining));

                                    lengths
                                }
                            };

                            let chunks = Layout::default()
                                .direction(direction.clone())
                                .constraints(lengths)
                                .split(user_chunk);

                            // loop through children and render
                            for (child, chunk) in children.iter().zip(chunks) {
                                match child {
                                    UserSplits::Panel(panel_i) => match panels.get(*panel_i) {
                                        None => (), // error
                                        Some((_, panel)) => {
                                            if *panel_i == active_panel {
                                                let (x, y) = panel.get_cursor(&chunk);
                                                f.set_cursor(x, y);
                                            }

                                            panel.make_widget(f, chunk);
                                        }
                                    },
                                    UserSplits::Split(split_i, children) => (), // recurse
                                }
                            }
                        }
                    }
                }
            }
        })?;

        match read()? {
            Event::Key(event) => {
                // CTRL operations global
                if event.modifiers.contains(KeyModifiers::CONTROL) {
                    // println!("control held");
                    match event.code {
                        KeyCode::Char('s') => {
                            // println!("s held");
                            // Split current panel
                            match panels.get(active_panel) {
                                None => {
                                    // println!("no active panel")
                                }
                                Some((split_i, panel)) => {
                                    match splits.get_mut(*split_i) {
                                        None => {
                                            // println!("no split on active panel")
                                        }
                                        Some(split) => {
                                            match split {
                                                UserSplits::Panel(_) => {
                                                    // println!("panel when expected split")
                                                } // shouldn't happen
                                                UserSplits::Split(direction, children) => {
                                                    let i = panels.len();
                                                    panels.push((
                                                        *split_i,
                                                        Box::new(TextEditPanel::new()),
                                                    ));

                                                    children.push(UserSplits::Panel(i));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        _ => (), // not an operation, ignore
                    }
                } else {
                    // defer to active panel
                    match panels.get_mut(active_panel) {
                        Some((_, panel)) => {
                            if !panel.receive_key(event) {
                                match event.code {
                                    KeyCode::Esc => break,
                                    _ => (),
                                }
                            }
                        }
                        None => (),
                    }
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
