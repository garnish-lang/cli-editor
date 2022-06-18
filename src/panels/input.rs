use crossterm::event::{KeyCode, KeyEvent};
use tui::layout::{Alignment, Rect};
use tui::style::{Color, Style};
use tui::text::{Span};
use tui::widgets::{Block, Paragraph, Wrap};

use crate::{EditorFrame, Panel};
use crate::app::StateChangeRequest;

pub struct InputPanel {
    id: char,
    min_x: u16,
    min_y: u16,
    cursor_x: u16,
    cursor_y: u16,
    text: String,
    title: String,
}

impl InputPanel {
    pub fn new() -> Self {
        InputPanel {
            id: '\0',
            cursor_x: 1,
            cursor_y: 1,
            min_x: 1,
            min_y: 1,
            text: String::new(),
            title: "Input".to_string(),
        }
    }
}

impl Panel for InputPanel {
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

    fn get_title(&self) -> &str {
        &self.title
    }

    fn set_title(&mut self, title: String) {
        self.title = title
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

    fn receive_key(&mut self, event: KeyEvent) -> (bool, Vec<StateChangeRequest>) {
        // temp ignore all modifiers
        if !event.modifiers.is_empty() {
            return (false, vec![]);
        }

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
            _ => return (false, vec![]),
        }

        (false, vec![])
    }
}
