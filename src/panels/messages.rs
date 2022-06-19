use std::process::id;
use crossterm::event::KeyEvent;
use tui::layout::{Alignment, Rect};
use tui::style::{Color, Style};
use tui::text::{Span, Spans};
use tui::widgets::{Block, Paragraph, Wrap};

use crate::app::{MessageChannel, StateChangeRequest};
use crate::{AppState, EditorFrame, Panel};

pub const MESSAGE_PANEL_TYPE_ID: &str = "Messages";

pub struct MessagesPanel {
    id: char,
}

impl MessagesPanel {
    pub fn new() -> Self {
        MessagesPanel { id: '\0' }
    }
}

impl Panel for MessagesPanel {
    fn type_id(&self) -> &str {
        MESSAGE_PANEL_TYPE_ID
    }

    fn make_widget(
        &self,
        app: &AppState,
        frame: &mut EditorFrame,
        rect: Rect,
        _is_active: bool,
        block: Block,
    ) {
        let spans: Vec<Span> = app
            .get_messages()
            .iter()
            .map(|m| {
                let color = match m.channel() {
                    MessageChannel::INFO => Color::White,
                    MessageChannel::WARNING => Color::Yellow,
                    MessageChannel::ERROR => Color::Red,
                };

                Span::styled(m.text(), Style::default().fg(color))
            })
            .collect();

        let para = Paragraph::new(Spans::from(spans))
            .block(block)
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true });

        frame.render_widget(para, rect);
    }

    fn get_title(&self) -> &str {
        "Messages"
    }

    fn set_id(&mut self, id: char) {
        self.id = id
    }

    fn get_id(&self) -> char {
        self.id
    }
}
