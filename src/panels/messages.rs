use tui::layout::Rect;
use tui::style::{Color, Style};
use tui::text::{Span, Text};
use tui::widgets::{Block, List, ListItem};

use crate::app::MessageChannel;
use crate::{AppState, EditorFrame, Panel};
use crate::panels::RenderDetails;

pub const MESSAGE_PANEL_TYPE_ID: &str = "Messages";

pub struct MessagesPanel {}

impl MessagesPanel {
    pub fn new() -> Self {
        MessagesPanel {}
    }
}

impl Panel for MessagesPanel {
    fn panel_type(&self) -> &str {
        MESSAGE_PANEL_TYPE_ID
    }

    fn make_widget(
        &self,
        app: &AppState,
        frame: &mut EditorFrame,
        rect: Rect,
        _is_active: bool,
        block: Block,
    ) -> RenderDetails {
        let spans: Vec<ListItem> = app
            .get_messages()
            .iter()
            .rev()
            .map(|m| {
                let color = match m.channel() {
                    MessageChannel::INFO => Color::White,
                    MessageChannel::WARNING => Color::Yellow,
                    MessageChannel::ERROR => Color::Red,
                };

                ListItem::new(Text::styled(
                    format!(" {}", m.text()),
                    Style::default().fg(color),
                ))
            })
            .collect();

        let list = List::new(spans)
            .block(block)
            .style(Style::default().fg(Color::White).bg(Color::Black));

        frame.render_widget(list, rect);

        RenderDetails::new(vec![], (1, 1))
    }

    fn make_title(&self, _state: &AppState) -> Vec<Span> {
        vec![Span::raw("Messages")]
    }

    fn get_cursor(&self) -> (u16, u16) {
        (1, 1)
    }
}
