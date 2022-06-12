use crossterm::event::{KeyCode, KeyEvent};

use tui::layout::{Alignment, Rect};
use tui::style::{Color, Style};
use tui::text::{Span, Text};
use tui::widgets::{Block, Paragraph, Wrap};

use crate::EditorFrame;

pub trait Panel {
    fn make_widget(&self, frame: &mut EditorFrame, rect: Rect, is_active: bool, block: Block);
    fn get_cursor(&self, rect: &Rect) -> (u16, u16);
    fn get_title(&self) -> String;
    fn get_length(&self) -> u16 {
        0
    }
    fn get_id(&self) -> char;
    fn set_id(&mut self, id: char);
    fn receive_key(&mut self, event: KeyEvent) -> bool;
    fn set_active(&mut self);
}

pub struct TextEditPanel {
    id: char,
    min_x: u16,
    min_y: u16,
    cursor_x: u16,
    cursor_y: u16,
    text: String,
}

impl TextEditPanel {
    pub fn new() -> Self {
        TextEditPanel {
            id: '\0',
            cursor_x: 1,
            cursor_y: 1,
            min_x: 1,
            min_y: 1,
            text: String::new(),
        }
    }
}

impl Panel for TextEditPanel {
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

    fn get_title(&self) -> String {
        "Editor".to_string()
    }

    fn get_id(&self) -> char {
        self.id
    }

    fn set_id(&mut self, id: char) {
        self.id = id;
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

pub struct PromptPanel {
    id: char,
    min_x: u16,
    min_y: u16,
    cursor_x: u16,
    cursor_y: u16,
    text: String,
}

impl PromptPanel {
    pub fn new() -> Self {
        PromptPanel {
            id: '\0',
            cursor_x: 1,
            cursor_y: 1,
            min_x: 1,
            min_y: 1,
            text: String::new(),
        }
    }
}

impl Panel for PromptPanel {
    fn make_widget(&self, frame: &mut EditorFrame, rect: Rect, _is_active: bool, block: Block) {
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

    fn get_title(&self) -> String {
        "Prompt".to_string()
    }

    fn get_length(&self) -> u16 {
        3
    }

    fn get_id(&self) -> char {
        self.id
    }

    fn set_id(&mut self, id: char) {
        self.id = id;
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
