use tui::layout::{Alignment, Rect};
use tui::style::{Color, Style};
use tui::text::{Span, Spans};
use tui::widgets::{Block, Paragraph, Wrap};
use crate::{AppState, EditorFrame, Panel};
use crate::app::MessageChannel;

pub struct MessagesPanel {

}

impl Panel for MessagesPanel {
    fn make_widget(&self, app: &AppState, frame: &mut EditorFrame, rect: Rect, _is_active: bool, block: Block) {
        let spans: Vec<Span> = app.get_messages().iter().map(|m| {
            let color = match m.channel() {
                MessageChannel::INFO => Color::White,
                MessageChannel::WARNING => Color::Yellow,
                MessageChannel::ERROR => Color::Red,
            };

            Span::styled(m.text(), Style::default().fg(color))
        }).collect();

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
}