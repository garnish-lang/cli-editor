use std::io;
use std::io::Stdout;

use crossterm::event::{read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, KeyEvent};
use crossterm::execute;
use crossterm::style::Print;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use tui::backend::CrosstermBackend;
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Style};
use tui::text::Text;
use tui::widgets::{Block, Borders, Paragraph, Widget, Wrap};
use tui::{Frame, Terminal};
use tui::buffer::Buffer;

struct AppState {
    active_panel: usize,
}

trait Panel {
    fn make_widget(&self, frame: &mut EditorFrame, rect: Rect);
    fn get_cursor(&self, rect: &Rect) -> (u16, u16);
    fn receive_key(&mut self, event: KeyEvent) -> bool;
    fn set_active(&mut self);
}

trait ChordReceiver {
    fn receive(
        &self,
        current_panel: &mut Box<dyn Panel>,
        app_state: &mut AppState,
        code: KeyCode,
    ) -> KeyInputState;
}

enum KeyInputState {
    Normal,
}

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
            _ => return false
        }

        true
    }

    fn set_active(&mut self) {
        todo!()
    }
}

type EditorFrame<'a> = Frame<'a, CrosstermBackend<Stdout>>;

fn main() -> Result<(), io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut edit_panel = TextEditPanel::new();

    loop {
        terminal.draw(|f| {
            let size = f.size();

            let layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(size.width)])
                .split(size);

            let (x, y) = edit_panel.get_cursor(&layout[0]);
            f.set_cursor(x, y);
            edit_panel.make_widget(f, layout[0]);
        })?;

        match read()? {
            Event::Key(event) => {
                if !edit_panel.receive_key(event) {
                    match event.code {
                        KeyCode::Esc => break,
                        _ => ()
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
