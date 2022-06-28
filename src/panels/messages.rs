use tui::layout::Rect;
use tui::style::{Color, Style};
use tui::text::{Span, Text};
use tui::widgets::{List, ListItem};

use crate::app::MessageChannel;
use crate::panels::RenderDetails;
use crate::{AppState, EditorFrame, Panel};

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

                ListItem::new(Text::styled(m.text().as_str(), Style::default().fg(color)))
            })
            .collect();

        let list = List::new(spans).style(Style::default().fg(Color::White).bg(Color::Black));

        frame.render_widget(list, rect);

        RenderDetails::new(vec![Span::raw("Messages")], (1, 1))
    }
}
