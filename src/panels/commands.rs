use crossterm::event::{KeyCode, KeyModifiers};
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::{Color, Style};
use tui::text::{Span, Spans, Text};
use tui::widgets::Paragraph;

use crate::app::StateChangeRequest;
use crate::commands::{CommandKey, Manager};
use crate::panels::text::RenderDetails;
use crate::{AppState, EditorFrame, TextPanel, CURSOR_MAX, CommandDetails};

pub(crate) fn render_handler(
    panel: &TextPanel,
    _state: &AppState,
    commands: &Manager,
    frame: &mut EditorFrame,
    rect: Rect,
) -> RenderDetails {
    let mut total_count = 0;

    let (selected_details, global_panel_spans) = match commands.current_global() {
        None => (None, vec![]),
        Some(command) => format_commands(panel, command, total_count),
    };

    total_count += global_panel_spans.len();

    let (current_panel_id, (current_selected_details, current_panel_spans)) = match commands.current_panel() {
        None => ("", (None, vec![])),
        Some((id, command)) => (id, format_commands(panel, command, total_count)),
    };

    let mut all_spans = vec![];

    if !global_panel_spans.is_empty() {
        all_spans.push(Spans::from(vec![Span::from(format!(
            "{:-<width$}",
            "Global Commands",
            width = rect.width as usize
        ))]));
        all_spans.extend(global_panel_spans);
        all_spans.push(Spans::default());
    }

    let current_panel_title = format!("{} Commands", current_panel_id);

    if !current_panel_spans.is_empty() {
        all_spans.push(Spans::from(vec![Span::from(format!(
            "{:-<width$}",
            current_panel_title,
            width = rect.width as usize
        ))]));
        all_spans.extend(current_panel_spans);
    }

    let commands_rect = match selected_details.or(current_selected_details) {
        Some(details) => {
            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![
                    Constraint::Length(rect.height - 10),
                    Constraint::Length(10),
                ])
                .split(rect);

            let spans = vec![
                Spans::from(Span::from(format!("{:=<width$}", details.name(), width=rect.width as usize))),
                Spans::from(details.description().as_str()),
            ];

            let para = Paragraph::new(Text::from(spans));

            frame.render_widget(para, layout[1]);

            layout[0]
        }
        None => rect,
    };

    let para = Paragraph::new(Text::from(all_spans))
        .style(Style::default().fg(Color::White).bg(Color::Black));

    frame.render_widget(para, commands_rect);

    RenderDetails::new("Commands".to_string(), CURSOR_MAX)
}

pub fn next_command(
    panel: &mut TextPanel,
    _code: KeyCode,
    _state: &mut AppState,
    commands: &mut Manager,
) -> (bool, Vec<StateChangeRequest>) {
    let count = match commands.current_panel() {
        Some(commands) => count_commands(commands.1),
        None => 0,
    } + match commands.current_global() {
        Some(command) => count_commands(command),
        None => 0,
    };

    if panel.selection() + 1 > count {
        panel.set_selection(1);
    } else {
        panel.set_selection(panel.selection() + 1);
    }

    (true, vec![])
}

pub fn previous_command(
    panel: &mut TextPanel,
    _code: KeyCode,
    _state: &mut AppState,
    commands: &mut Manager,
) -> (bool, Vec<StateChangeRequest>) {
    let count = match commands.current_panel() {
        Some(commands) => count_commands(commands.1),
        None => 0,
    } + match commands.current_global() {
        Some(command) => count_commands(command),
        None => 0,
    };

    if panel.selection() <= 1 {
        panel.set_selection(count);
    } else {
        panel.set_selection(panel.selection() - 1);
    }

    (true, vec![])
}

pub fn deselect(
    panel: &mut TextPanel,
    _code: KeyCode,
    _state: &mut AppState,
    _commands: &mut Manager,
) -> (bool, Vec<StateChangeRequest>) {
    panel.set_selection(0);

    (true, vec![])
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

fn format_commands<'a, T>(panel: &'a TextPanel, command: &'a CommandKey<T>, total_count: usize) -> (Option<&'a CommandDetails>, Vec<Spans<'a>>) {
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

    let mut selected = None;

    let items = items
        .iter()
        .enumerate()
        .map(|(i, (details, span))| {
            let style = match panel.selection() {
                0 => Style::default(),
                n => match total_count + i == n - 1 {
                    true => {
                        selected = Some(*details);
                        Style::default().bg(Color::DarkGray)
                    }
                    false => Style::default(),
                },
            };

            Spans::from(vec![
                Span::styled(
                    format!("{:<width$}", details.name(), width = name_length),
                    style,
                ),
                Span::styled(" | ", style),
                Span::styled(span.clone(), style),
            ])
        })
        .collect();

    (selected, items)
}

fn count_commands<T>(root: &CommandKey<T>) -> usize {
    let mut count = 0;
    let mut stack = vec![root];

    while let Some(command) = stack.pop() {
        match command {
            CommandKey::Node(_, _, children, _) => {
                for value in children.values() {
                    stack.push(value);
                }
            }
            CommandKey::Leaf(..) => {
                count += 1;
            }
        }
    }

    count
}
