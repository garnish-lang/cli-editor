use tui::layout::Rect;
use tui::style::{Color, Style};
use tui::text::{Span, Spans, Text};
use tui::widgets::Paragraph;

use crate::commands::{CommandKey, Manager};
use crate::panels::text::RenderDetails;
use crate::{AppState, EditorFrame, TextPanel, CURSOR_MAX};

pub(crate) fn render_handler(
    panel: &TextPanel,
    _state: &AppState,
    commands: &Manager,
    frame: &mut EditorFrame,
    rect: Rect,
) -> RenderDetails {
    let mut items = vec![];
    match commands.current_panel() {
        None => (),
        Some(command) => {
            let mut stack = vec![(0, command)];
            while let Some((depth, command)) = stack.pop() {
                items.push((depth, command));

                match command {
                    CommandKey::Node(_, _, children, _) => {
                        for value in children.values() {
                            stack.push((depth + 1, value));
                        }
                    }
                    CommandKey::Leaf(..) => (),
                }
            }
        }
    }

    let mut spans = vec![];
    for (depth, item) in items {
        match item {
            CommandKey::Node(..) => (),
            CommandKey::Leaf(_, _, details, _) => {
                spans.push(Spans::from(vec![
                    Span::from(details.name().as_str())
                ]));
            }
        }
    }

    let para = Paragraph::new(Text::from(spans))
        .style(Style::default().fg(Color::White).bg(Color::Black));

    frame.render_widget(para, rect);

    RenderDetails::new("Commands".to_string(), CURSOR_MAX)
}
