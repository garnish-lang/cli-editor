use std::fs;
use crossterm::event::{KeyCode, KeyEvent};
use tui::layout::{Alignment, Rect};
use tui::style::{Color, Style};
use tui::text::Text;
use tui::widgets::{Block, Paragraph, Wrap};

use crate::{AppState, catch_all, CommandDetails, CommandKeyId, Commands, ctrl_key, EditorFrame, Panel};
use crate::app::StateChangeRequest;
use crate::commands::shift_catch_all;

pub struct TextEditPanel {
    id: char,
    min_x: u16,
    min_y: u16,
    cursor_x: u16,
    cursor_y: u16,
    text: String,
    title: String,
    commands: Commands<EditCommand>
}

#[allow(dead_code)]
impl TextEditPanel {
    pub fn new() -> Self {
        TextEditPanel {
            id: '\0',
            cursor_x: 1,
            cursor_y: 1,
            min_x: 1,
            min_y: 1,
            text: String::new(),
            title: "Editor".to_string(),
            commands: Commands::<EditCommand>::new()
        }
    }

    pub fn get_text(&self) -> &str {
        &self.text
    }

    pub fn set_text(&mut self, text: String) {
        self.text = text;
    }

    fn handle_key_stroke(&mut self, code: KeyCode) -> (bool, Vec<StateChangeRequest>) {
        match code {
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
            _ => return (false, vec![]),
        }

        (true, vec![])
    }

    fn open_file(&mut self, _code: KeyCode) -> (bool, Vec<StateChangeRequest>) {
        (false, vec![StateChangeRequest::input_request("File Name".to_string())])
    }

    fn set_cursor_to_end(&mut self) {
        self.cursor_x = self.min_x;
        self.cursor_y = self.min_y;

        for c in self.text.chars() {
            match c {
                '\n' => {
                    self.cursor_x = self.min_x;
                    self.cursor_y += 1;
                }
                _ => {
                    self.cursor_x += 1;
                }
            }
        }
    }
}

impl Panel for TextEditPanel {
    fn init(&mut self, state: &mut AppState) {
        match make_commands() {
            Ok(commands) => self.commands = commands,
            Err(e) => state.add_error(e)
        }
    }

    fn make_widget(&self, frame: &mut EditorFrame, rect: Rect, _is_active: bool, block: Block) {
        let para_text = Text::from(self.text.clone());
        let para = Paragraph::new(para_text)
            .block(block)
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true });

        frame.render_widget(para, rect);
    }

    fn get_cursor(&self, rect: &Rect) -> (u16, u16) {
        (rect.x + self.cursor_x, rect.y + self.cursor_y)
    }

    fn get_title(&self) -> &str {
        &self.title
    }

    fn set_title(&mut self, title: String) {
        self.title = title
    }

    fn get_id(&self) -> char {
        self.id
    }

    fn set_id(&mut self, id: char) {
        self.id = id;
    }

    fn receive_key(&mut self, event: KeyEvent) -> (bool, Vec<StateChangeRequest>) {
        let (end, action) = self.commands.advance(CommandKeyId::new(event.code, event.modifiers));

        if end {
            self.commands.reset();
        }

        match action {
            Some(a) => a(self, event.code),
            None => (!end, vec![])
        }
    }

    fn receive_input(&mut self, input: String) -> Vec<StateChangeRequest> {
        let mut changes = vec![];

        match fs::read_to_string(input) {
            Err(e) => changes.push(StateChangeRequest::error(e)),
            Ok(text) => self.text = text
        }

        self.set_cursor_to_end();

        changes
    }
}

type EditCommand = fn(&mut TextEditPanel, KeyCode) -> (bool, Vec<StateChangeRequest>);

pub fn make_commands() -> Result<Commands<EditCommand>, String> {
    let mut commands = Commands::<EditCommand>::new();

    commands.insert(|b| {
        b.node(catch_all()).action(CommandDetails::empty(), TextEditPanel::handle_key_stroke)
    })?;

    commands.insert(|b| {
        b.node(shift_catch_all()).action(CommandDetails::empty(), TextEditPanel::handle_key_stroke)
    })?;

    commands.insert(|b| {
        b.node(ctrl_key('o')).action(CommandDetails::open_file(), TextEditPanel::open_file)
    })?;

    Ok(commands)
}