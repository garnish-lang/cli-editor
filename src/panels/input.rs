use crossterm::event::{KeyCode, KeyEvent};
use tui::layout::{Alignment, Rect};
use tui::style::{Color, Style};
use tui::text::{Span};
use tui::widgets::{Block, Paragraph, Wrap};

use crate::{AppState, catch_all, CommandDetails, CommandKeyId, Commands, EditorFrame, Panel};
use crate::app::StateChangeRequest;
use crate::commands::shift_catch_all;

pub const INPUT_PANEL_TYPE_ID: &str = "Input";

pub struct InputPanel {
    min_x: u16,
    min_y: u16,
    cursor_x: u16,
    cursor_y: u16,
    text: String,
    title: String,
    commands: Commands<InputCommand>,
    length: u16,
    visible: bool,
}

impl InputPanel {
    pub fn new() -> Self {
        InputPanel {
            cursor_x: 1,
            cursor_y: 1,
            min_x: 1,
            min_y: 1,
            text: String::new(),
            title: "".to_string(),
            commands: Commands::<InputCommand>::new(),
            length: 3,
            visible: false
        }
    }

    #[allow(dead_code)]
    pub fn set_text(&mut self, text: String) {
        self.text = text;
    }

    fn handle_key_stroke(&mut self, code: KeyCode) -> (bool, Vec<StateChangeRequest>) {
        let mut requests = vec![];
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
                requests.push(StateChangeRequest::input_complete(self.text.clone()));
                self.text = String::new();
                // self.text.push('\n');
                // self.cursor_y += 1;
                // self.cursor_x = 1;
            }
            KeyCode::Char(c) => {
                self.cursor_x += 1;
                self.text.push(c);
            }
            _ => return (false, vec![]),
        }

        (true, requests)
    }
}

impl Panel for InputPanel {
    fn type_id(&self) -> &str {
        INPUT_PANEL_TYPE_ID
    }

    fn init(&mut self, state: &mut AppState) {
        match make_commands() {
            Ok(commands) => self.commands = commands,
            Err(e) => state.add_error(e)
        }
    }

    fn make_widget(&self, _state: &AppState, frame: &mut EditorFrame, rect: Rect, _is_active: bool, block: Block) {
        let para_text = Span::from(self.text.clone());

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

    fn get_length(&self) -> u16 {
        self.length
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

    fn show(&mut self) {
        self.visible = true;
    }

    fn hide(&mut self) {
        self.visible = false;
    }

    fn visible(&self) -> bool {
        self.visible
    }
}

type InputCommand = fn(&mut InputPanel, KeyCode) -> (bool, Vec<StateChangeRequest>);

pub fn make_commands() -> Result<Commands<InputCommand>, String> {
    let mut commands = Commands::<InputCommand>::new();

    commands.insert(|b| {
        b.node(catch_all()).action(CommandDetails::empty(), InputPanel::handle_key_stroke)
    })?;

    commands.insert(|b| {
        b.node(shift_catch_all()).action(CommandDetails::empty(), InputPanel::handle_key_stroke)
    })?;

    Ok(commands)
}