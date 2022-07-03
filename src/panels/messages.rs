use tui::layout::Rect;
use tui::style::{Color, Style};
use tui::text::{Span, Text};
use tui::widgets::{List, ListItem};

use crate::app::MessageChannel;
use crate::{AppState, CURSOR_MAX, EditorFrame, TextPanel};
use crate::panels::text::RenderDetails;

pub struct MessagesPanel {}

impl MessagesPanel {
    pub fn render_handler(_: &TextPanel, state: &AppState, frame: &mut EditorFrame, rect: Rect) -> RenderDetails {
        let spans: Vec<ListItem> = state
            .get_messages()
            .iter()
            .rev()
            .map(|m| {
                let color = match m.channel() {
                    MessageChannel::INFO => Color::White,
                    MessageChannel::WARNING => Color::Yellow,
                    MessageChannel::ERROR => Color::Red,
                };

                ListItem::new(Text::styled(m.text().as_str(), Style::default().fg(color)))
            })
            .collect();

        let list = List::new(spans).style(Style::default().fg(Color::White).bg(Color::Black));

        frame.render_widget(list, rect);

        RenderDetails::new("Messages".to_string(), CURSOR_MAX)
    }
}