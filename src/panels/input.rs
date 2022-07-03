use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Style};
use tui::text::{Span, Spans, Text};
use tui::widgets::Paragraph;

use crate::app::StateChangeRequest;
use crate::commands::{alt_catch_all, code, shift_catch_all};
use crate::{catch_all, AppState, CommandDetails, CommandKeyId, Commands, EditorFrame, TextPanel};

pub struct InputPanel {
    cursor_index: usize,
    text: String,
    // commands: Commands<InputCommand>,
    visible: bool,
    quick_select: usize,
    continuation_marker: String,
}

impl InputPanel {
    pub fn new() -> Self {
        InputPanel {
            cursor_index: 0,
            text: String::new(),
            // commands: Commands::<InputCommand>::new(),
            visible: false,
            quick_select: 0,
            continuation_marker: "... ".to_string(),
        }
    }

    #[allow(dead_code)]
    pub fn set_text(&mut self, text: String) {
        self.text = text;
    }

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

    // fn handle_key_stroke(
    //     panel: &TextPanel,
    //     code: KeyCode,
    //     _state: &mut AppState,
    // ) -> (bool, Vec<StateChangeRequest>) {
    //     let mut requests = vec![];
    //     match code {
    //         KeyCode::Backspace => match panel.text.pop() {
    //             None => {
    //                 panel.cursor_index = 0;
    //             }
    //             Some(_) => {
    //                 panel.cursor_index -= 1;
    //             }
    //         },
    //         KeyCode::Delete => {
    //             // ??
    //         }
    //         KeyCode::Enter => {
    //             requests.push(StateChangeRequest::input_complete(panel.text.clone()));
    //             panel.text = String::new();
    //             panel.cursor_index = 0;
    //         }
    //         KeyCode::Char(c) => {
    //             panel.cursor_index += 1;
    //             panel.text.push(c);
    //         }
    //         _ => return (false, vec![]),
    //     }
    //
    //     (true, requests)
    // }

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
        match state.input_request().and_then(|r| r.completer()) {
            None => (),
            Some(completer) => {
                let options = completer.get_options(panel.text().as_str());
                match options.get(panel.selection()) {
                    // reset quick select to start
                    None => panel.set_selection(0),
                    Some(selection) => {
                        panel.append_text(selection.remaining());
                        panel.set_selection(1 + selection.remaining().len());
                    }
                }
            }
        }

        (false, vec![])
    }

    // pub fn make_title(_: &TextPanel, state: &AppState) -> Vec<Span> {
    //     match state.input_request() {
    //         Some(request) => {
    //             vec![Span::raw(request.prompt().clone())]
    //         }
    //         None => vec![],
    //     }
    // }

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
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyCode;

    use crate::app::StateChangeRequest;
    use crate::autocomplete::{AutoCompleter, Completion};
    use crate::commands::Manager;
    use crate::{AppState, InputPanel, Panels, TextPanel};

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
            &mut panels,
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
            &mut panels,
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
            &mut panels,
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
            &mut panels,
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
            &mut panels,
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
            &mut panels,
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
            &mut panels,
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
            &mut panels,
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
            &mut panels,
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
            &mut panels,
        );

        let mut input = TextPanel::input_panel();
        input.set_text("ca".to_string());
        input.set_selection(9);

        InputPanel::fill_current_quick_select(&mut input, KeyCode::Null, &mut state);

        assert_eq!(input.text(), "ca".to_string());
        assert_eq!(input.selection(), 0);
    }
}
