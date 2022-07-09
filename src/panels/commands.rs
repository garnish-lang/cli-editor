use crossterm::event::{KeyCode, KeyModifiers};
use tui::layout::{Constraint, Layout, Rect};
use tui::style::{Color, Style};
use tui::text::{Span, Spans, Text};
use tui::widgets::Paragraph;

use crate::commands::{CommandKey, Manager};
use crate::panels::text::RenderDetails;
use crate::{AppState, EditorFrame, TextPanel, CURSOR_MAX};

pub(crate) fn render_handler(
    _panel: &TextPanel,
    _state: &AppState,
    commands: &Manager,
    frame: &mut EditorFrame,
    rect: Rect,
) -> RenderDetails {
    let (current_panel_id, current_panel_spans) = match commands.current_panel() {
        None => ("", vec![]),
        Some((id, command)) => (id, format_commands(command)),
    };

    let global_panel_spans = match commands.current_global() {
        None => vec![],
        Some(command) => format_commands(command),
    };

    let mut all_spans = vec![];

    if !global_panel_spans.is_empty() {
        all_spans.push(Spans::from(vec![Span::from(format!("{:-<width$}", "Global Commands", width=rect.width as usize))]));
        all_spans.extend(global_panel_spans);
        all_spans.push(Spans::default());
    }

    let current_panel_title = format!("{} Commands", current_panel_id);

    if !current_panel_spans.is_empty() {
        all_spans.push(Spans::from(vec![
            Span::from(format!("{:-<width$}", current_panel_title, width=rect.width as usize))
        ]));
        all_spans.extend(current_panel_spans);
    }

    let para = Paragraph::new(Text::from(all_spans))
        .style(Style::default().fg(Color::White).bg(Color::Black));

    frame.render_widget(para, rect);

    RenderDetails::new("Commands".to_string(), CURSOR_MAX)
}

fn format_modifiers(modifiers: KeyModifiers) -> &'static str {
    match (
        modifiers.contains(KeyModifiers::ALT),
        modifiers.contains(KeyModifiers::CONTROL),
        modifiers.contains(KeyModifiers::SHIFT),
    ) {
        (false, false, false) => "",
        (true, false, false) => "ALT",
        (false, true, false) => "CTRL",
        (false, false, true) => "SHIFT",
        (true, true, false) => "CTRL + ALT",
        (true, false, true) => "SHIFT + ALT",
        (false, true, true) => "SHIFT + CTRL",
        (true, true, true) => "SHIFT + CTRL + ALT",
    }
}

fn format_modifiers_concise(modifiers: KeyModifiers) -> &'static str {
    match (
        modifiers.contains(KeyModifiers::ALT),
        modifiers.contains(KeyModifiers::CONTROL),
        modifiers.contains(KeyModifiers::SHIFT),
    ) {
        (false, false, false) => "",
        (true, false, false) => "A",
        (false, true, false) => "C",
        (false, false, true) => "S",
        (true, true, false) => "CA",
        (true, false, true) => "SA",
        (false, true, true) => "SC",
        (true, true, true) => "SCA",
    }
}

fn format_code(code: KeyCode) -> String {
    match code {
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Null => "*".to_string(),
        c => format!("{:?}", c),
    }
}

fn format_commands<T>(command: &CommandKey<T>) -> Vec<Spans> {
    let mut items = vec![];

    let mut name_length = 0;

    let mut stack = vec![(0, "".to_string(), command)];
    while let Some((depth, base, command)) = stack.pop() {
        match command {
            CommandKey::Node(code, modifiers, children, _) => {
                let base = match depth == 0 {
                    true => base,
                    false => {
                        let our_str = match modifiers.is_empty() {
                            true => format_code(*code),
                            false => format!(
                                "{} + {}",
                                format_modifiers_concise(*modifiers),
                                format_code(*code)
                            ),
                        };

                        match base.is_empty() {
                            true => our_str,
                            false => format!("{} -> {}", base, our_str),
                        }
                    }
                };

                for value in children.values() {
                    stack.push((depth + 1, base.to_string(), value));
                }
            }
            CommandKey::Leaf(code, modifiers, details, _) => {
                let our_str = match modifiers.is_empty() {
                    true => format_code(*code),
                    false => format!(
                        "{} + {}",
                        format_modifiers_concise(*modifiers),
                        format_code(*code)
                    ),
                };

                let base = match base.is_empty() {
                    true => our_str,
                    false => format!("{} -> {}", base, our_str),
                };

                if details.name().len() > name_length {
                    name_length = details.name().len();
                }

                // push entire command to spans
                items.push((details, base));
            }
        }
    }

    items.sort_by(|item, item2| item.0.name().cmp(item2.0.name()));

    items
        .iter()
        .map(|(details, span)| {
            Spans::from(vec![
                Span::from(format!("{:<width$}", details.name(), width = name_length)),
                Span::from(" | "),
                Span::from(span.clone()),
            ])
        })
        .collect()
}
