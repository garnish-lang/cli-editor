use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Style};
use tui::text::{Span, Spans, Text};
use tui::widgets::{Block, Paragraph};

use crate::app::StateChangeRequest;
use crate::commands::{alt_catch_all, code, Manager, shift_catch_all};
use crate::{catch_all, AppState, CommandDetails, CommandKeyId, Commands, EditorFrame, TextPanel, CURSOR_MAX};
use crate::panels::text::RenderDetails;

pub struct InputPanel {}

impl InputPanel {
    pub(crate) fn handle_key_stroke(
        panel: &mut TextPanel,
        code: KeyCode,
        state: &mut AppState,
    ) -> (bool, Vec<StateChangeRequest>) {
        panel.handle_key_stroke_internal(code, state, InputPanel::submit_input)
    }

    pub fn submit_input(panel: &mut TextPanel, changes: &mut Vec<StateChangeRequest>) {
        changes.push(StateChangeRequest::input_complete(panel.text().clone()));
        panel.set_text("");
        panel.set_selection(0);
    }

    pub fn next_quick_select(
        panel: &mut TextPanel,
        _code: KeyCode,
        state: &mut AppState,
    ) -> (bool, Vec<StateChangeRequest>) {
        match state.input_request().and_then(|r| r.completer()) {
            None => (),
            Some(completer) => {
                let option_count = completer.get_options(panel.text().as_str()).len();

                panel.set_selection(panel.selection() + 1);
                if panel.selection() >= option_count {
                    panel.set_selection(0);
                }
            }
        }

        (false, vec![])
    }

    pub fn previous_quick_select(
        panel: &mut TextPanel,
        _code: KeyCode,
        state: &mut AppState,
    ) -> (bool, Vec<StateChangeRequest>) {
        match state.input_request().and_then(|r| r.completer()) {
            None => (),
            Some(completer) => {
                let option_count = completer.get_options(panel.text().as_str()).len();

                panel.set_selection(if panel.selection() == 0 {
                    option_count - 1
                } else {
                    panel.selection() - 1
                });
            }
        }

        (false, vec![])
    }

    pub fn fill_quick_select(
        panel: &mut TextPanel,
        code: KeyCode,
        state: &mut AppState,
    ) -> (bool, Vec<StateChangeRequest>) {
        state.add_info("Filling");
        match state.input_request().and_then(|r| r.completer()) {
            None => (),
            Some(completer) => {
                let options = completer.get_options(panel.text().as_str());
                let input = match code {
                    KeyCode::Char(c) => {
                        if ('1'..'9').contains(&c) {
                            c as usize - '1' as usize
                        } else {
                            return (false, vec![]);
                        }
                    }
                    _ => return (false, vec![]),
                };

                match options.get(input) {
                    Some(selection) => {
                        panel.append_text(selection.remaining());
                        panel.set_cursor_index(
                            panel.cursor_index_in_line() + selection.remaining().len(),
                        );
                    }
                    None => return (false, vec![]),
                }
            }
        }

        (false, vec![])
    }

    pub fn fill_current_quick_select(
        panel: &mut TextPanel,
        _code: KeyCode,
        state: &mut AppState,
    ) -> (bool, Vec<StateChangeRequest>) {
        state.add_info("Filling current");
        match state.input_request().and_then(|r| r.completer()) {
            None => (),
            Some(completer) => {
                let options = completer.get_options(panel.text().as_str());
                match options.get(panel.selection()) {
                    // reset quick select to start
                    None => panel.set_selection(0),
                    Some(selection) => {
                        panel.append_text(selection.remaining());
                        panel.set_cursor_index(panel.cursor_index_in_line() + selection.remaining().len());
                    }
                }
            }
        }

        (false, vec![])
    }

    pub fn length_handler(
        panel: &TextPanel,
        fixed_length: u16,
        _flex_length: u16,
        _direction: Direction,
        state: &AppState,
    ) -> u16 {
        // minus 2 because of borders
        let max_text_length = fixed_length - 2;
        let continuation_length =
            max_text_length - panel.continuation_marker().len().try_into().unwrap_or(0);
        let continuation_lines = if panel.text().len() >= max_text_length.into() {
            let remaining_length = panel.text().len() as u16 - max_text_length;
            // remaining length will be 0 or more
            // need at least one line to display cursor on next line if current is full
            // remaining line count will be number of continuation lines - 1 (due to integer division)
            1 + remaining_length / continuation_length
        } else {
            0
        };

        // base is 1 line plus 2 for borders
        // plus additional 2 if completion will be showing, 1 for border and 1 for completion text

        state
            .input_request()
            .and_then(|r| r.completer())
            .map(|_| 5)
            .unwrap_or(3)
            + continuation_lines
    }

    pub fn render_handler(panel: &TextPanel, state: &AppState, _: &Manager, frame: &mut EditorFrame, rect: Rect) -> RenderDetails {
        let line_count = panel.lines().len();
        let line_count_size = line_count.to_string().len().min(u16::MAX as usize) as u16;

        let (complete_text, has_completer, prompt) = match state.input_request().and_then(|r| Some((r.prompt(), r.completer())))
        {
            Some((prompt, Some(completer))) => (
                completer
                    .get_options(panel.text().as_str())
                    .iter()
                    .take(9)
                    .enumerate()
                    .map(|(i, option)| {
                        vec![
                            Span::styled(
                                format!("{} {}", i + 1, option.option()),
                                Style::default()
                                    .fg(match i % 2 {
                                        0 => Color::Cyan,
                                        1 => Color::Magenta,
                                        _ => Color::White,
                                    })
                                    .bg(match panel.selection() == i {
                                        true => Color::Gray,
                                        false => Color::Black,
                                    }),
                            ),
                            Span::raw(" "),
                        ]
                    })
                    .flatten()
                    .collect::<Vec<Span>>(),
                true,
                Some(prompt),
            ),
            _ => (vec![], false, None),
        };

        let text_layout = if has_completer {
            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![
                    Constraint::Length(rect.height - 2),
                    Constraint::Length(1),
                    Constraint::Length(1),
                ])
                .split(rect);

            // render completion here since we're already in check
            let divider = Paragraph::new(Span::from("-".repeat(rect.width as usize)))
                .alignment(Alignment::Center);

            let complete_para = Paragraph::new(Spans::from(complete_text))
                .style(Style::default().fg(Color::White).bg(Color::Black))
                .alignment(Alignment::Left);

            frame.render_widget(divider, layout[1]);
            frame.render_widget(complete_para, layout[2]);

            layout[0]
        } else {
            rect
        };

        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Length(line_count_size),
                Constraint::Length(panel.gutter_size()),
                Constraint::Length(rect.width - line_count_size - panel.gutter_size()),
            ])
            .split(text_layout);

        let gutter_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Length(1),
                Constraint::Length(panel.gutter_size() - 2),
                Constraint::Length(1),
            ])
            .split(layout[1]);

        let (lines, cursor, gutter) = panel.make_text_content(layout[2]);

        let para_text = Text::from(lines);

        let line_numbers_para = Paragraph::new(Text::from(gutter)).alignment(Alignment::Right);

        frame.render_widget(line_numbers_para, layout[0]);

        let gutter = Block::default().style(Style::default().bg(Color::DarkGray));

        frame.render_widget(gutter, gutter_layout[1]);

        let para =
            Paragraph::new(para_text).style(Style::default().fg(Color::White).bg(Color::Black));

        frame.render_widget(para, layout[2]);

        return RenderDetails::new(prompt.unwrap_or(panel.title()).to_string(), cursor)
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyCode;

    use crate::app::StateChangeRequest;
    use crate::autocomplete::{AutoCompleter, Completion};
    use crate::commands::Manager;
    use crate::{AppState, Panels, TextPanel};
    use crate::panels::input::InputPanel;

    pub struct TestCompleter {}

    impl AutoCompleter for TestCompleter {
        fn get_options(&self, s: &str) -> Vec<Completion> {
            ["shout", "shells", "sell", "cats", "capture"]
                .iter()
                .filter(|o| o.starts_with(s))
                .map(|o| Completion::new(o.to_string(), o[s.len()..].to_string()))
                .collect()
        }
    }

    #[test]
    fn next_quick_select() {
        let mut panels = Panels::new();
        let mut state = AppState::new();
        let mut commands = Manager::default();
        state.init(&mut panels, &mut commands);
        state.handle_changes(
            vec![StateChangeRequest::Input(
                "Test".to_string(),
                Some(Box::new(TestCompleter {})),
            )],
            &mut panels, &mut commands
        );

        let mut input = TextPanel::input_panel();

        InputPanel::next_quick_select(&mut input, KeyCode::Null, &mut state);

        assert_eq!(input.selection(), 1);
    }

    #[test]
    fn next_quick_select_past_options() {
        let mut panels = Panels::new();
        let mut state = AppState::new();
        let mut commands = Manager::default();
        state.init(&mut panels, &mut commands);
        state.handle_changes(
            vec![StateChangeRequest::Input(
                "Test".to_string(),
                Some(Box::new(TestCompleter {})),
            )],
            &mut panels, &mut commands
        );

        let mut input = TextPanel::input_panel();
        input.set_selection(4);

        InputPanel::next_quick_select(&mut input, KeyCode::Null, &mut state);

        assert_eq!(input.selection(), 0);
    }

    #[test]
    fn previous_quick_select() {
        let mut panels = Panels::new();
        let mut state = AppState::new();
        let mut commands = Manager::default();
        state.init(&mut panels, &mut commands);
        state.handle_changes(
            vec![StateChangeRequest::Input(
                "Test".to_string(),
                Some(Box::new(TestCompleter {})),
            )],
            &mut panels, &mut commands
        );

        let mut input = TextPanel::input_panel();
        input.set_selection(3);

        InputPanel::previous_quick_select(&mut input, KeyCode::Null, &mut state);

        assert_eq!(input.selection(), 2);
    }

    #[test]
    fn previous_quick_select_past_options() {
        let mut panels = Panels::new();
        let mut state = AppState::new();
        let mut commands = Manager::default();
        state.init(&mut panels, &mut commands);
        state.handle_changes(
            vec![StateChangeRequest::Input(
                "Test".to_string(),
                Some(Box::new(TestCompleter {})),
            )],
            &mut panels, &mut commands
        );

        let mut input = TextPanel::input_panel();
        input.set_selection(0);

        InputPanel::previous_quick_select(&mut input, KeyCode::Null, &mut state);

        assert_eq!(input.selection(), 4);
    }

    #[test]
    fn fill_quick_select() {
        let mut panels = Panels::new();
        let mut state = AppState::new();
        let mut commands = Manager::default();
        state.init(&mut panels, &mut commands);
        state.handle_changes(
            vec![StateChangeRequest::Input(
                "Test".to_string(),
                Some(Box::new(TestCompleter {})),
            )],
            &mut panels, &mut commands
        );

        let mut input = TextPanel::input_panel();
        input.set_text("se".to_string());

        InputPanel::fill_quick_select(&mut input, KeyCode::Char('1'), &mut state);

        assert_eq!(input.text(), "sell".to_string());
    }

    #[test]
    fn fill_quick_select_invalid_selection() {
        let mut panels = Panels::new();
        let mut state = AppState::new();
        let mut commands = Manager::default();
        state.init(&mut panels, &mut commands);
        state.handle_changes(
            vec![StateChangeRequest::Input(
                "Test".to_string(),
                Some(Box::new(TestCompleter {})),
            )],
            &mut panels, &mut commands
        );

        let mut input = TextPanel::input_panel();
        input.set_text("se".to_string());

        InputPanel::fill_quick_select(&mut input, KeyCode::Char('0'), &mut state);

        assert_eq!(input.text(), "se".to_string());
    }

    #[test]
    fn fill_quick_select_invalid_code() {
        let mut panels = Panels::new();
        let mut state = AppState::new();
        let mut commands = Manager::default();
        state.init(&mut panels, &mut commands);
        state.handle_changes(
            vec![StateChangeRequest::Input(
                "Test".to_string(),
                Some(Box::new(TestCompleter {})),
            )],
            &mut panels, &mut commands
        );

        let mut input = TextPanel::input_panel();
        input.set_text("se".to_string());

        InputPanel::fill_quick_select(&mut input, KeyCode::Enter, &mut state);

        assert_eq!(input.text(), "se".to_string());
    }

    #[test]
    fn fill_quick_select_out_of_option_range() {
        let mut panels = Panels::new();
        let mut state = AppState::new();
        let mut commands = Manager::default();
        state.init(&mut panels, &mut commands);
        state.handle_changes(
            vec![StateChangeRequest::Input(
                "Test".to_string(),
                Some(Box::new(TestCompleter {})),
            )],
            &mut panels, &mut commands
        );

        let mut input = TextPanel::input_panel();
        input.set_text("se".to_string());

        InputPanel::fill_quick_select(&mut input, KeyCode::Char('9'), &mut state);

        assert_eq!(input.text(), "se".to_string());
    }

    #[test]
    fn fill_current_quick_select() {
        let mut panels = Panels::new();
        let mut state = AppState::new();
        let mut commands = Manager::default();
        state.init(&mut panels, &mut commands);
        state.handle_changes(
            vec![StateChangeRequest::Input(
                "Test".to_string(),
                Some(Box::new(TestCompleter {})),
            )],
            &mut panels, &mut commands
        );

        let mut input = TextPanel::input_panel();
        input.set_text("ca".to_string());
        input.set_selection(1);

        InputPanel::fill_current_quick_select(&mut input, KeyCode::Null, &mut state);

        assert_eq!(input.text(), "capture".to_string());
    }

    #[test]
    fn fill_current_quick_select_out_of_range() {
        let mut panels = Panels::new();
        let mut state = AppState::new();
        let mut commands = Manager::default();
        state.init(&mut panels, &mut commands);
        state.handle_changes(
            vec![StateChangeRequest::Input(
                "Test".to_string(),
                Some(Box::new(TestCompleter {})),
            )],
            &mut panels, &mut commands
        );

        let mut input = TextPanel::input_panel();
        input.set_text("ca".to_string());
        input.set_selection(9);

        InputPanel::fill_current_quick_select(&mut input, KeyCode::Null, &mut state);

        assert_eq!(input.text(), "ca".to_string());
        assert_eq!(input.selection(), 0);
    }
}
