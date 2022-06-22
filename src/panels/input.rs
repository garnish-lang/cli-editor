use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Style};
use tui::text::{Span, Spans};
use tui::widgets::{Block, Paragraph};

use crate::app::StateChangeRequest;
use crate::commands::{code, alt_catch_all, shift_catch_all};
use crate::{catch_all, AppState, CommandDetails, CommandKeyId, Commands, EditorFrame, Panel};

pub const INPUT_PANEL_TYPE_ID: &str = "Input";

pub struct InputPanel {
    min_x: u16,
    min_y: u16,
    cursor_x: u16,
    cursor_y: u16,
    text: String,
    commands: Commands<InputCommand>,
    visible: bool,
    quick_select: usize,
}

impl InputPanel {
    pub fn new() -> Self {
        InputPanel {
            cursor_x: 1,
            cursor_y: 1,
            min_x: 1,
            min_y: 1,
            text: String::new(),
            commands: Commands::<InputCommand>::new(),
            visible: false,
            quick_select: 0,
        }
    }

    #[allow(dead_code)]
    pub fn set_text(&mut self, text: String) {
        self.text = text;
    }

    fn handle_key_stroke(
        &mut self,
        code: KeyCode,
        _state: &mut AppState,
    ) -> (bool, Vec<StateChangeRequest>) {
        let mut requests = vec![];
        match code {
            KeyCode::Backspace => {
                match self.text.pop() {
                    None => {
                        self.cursor_x = self.min_x;
                        self.cursor_y = self.min_y;
                    }
                    Some(c) => {
                        match c {
                            '\n' => {
                                self.cursor_y -= 1;
                                self.cursor_x = self.min_x;

                                // count from back until a newline is reached
                                for c in self.text.chars().rev() {
                                    if c == '\n' {
                                        break;
                                    }
                                    self.cursor_x += 1;
                                }
                            }
                            _ => {
                                self.cursor_x -= 1;
                            }
                        }
                    }
                }
            }
            KeyCode::Delete => {
                // ??
            }
            KeyCode::Enter => {
                requests.push(StateChangeRequest::input_complete(self.text.clone()));
                self.text = String::new();
                self.cursor_x = self.min_x;
                self.cursor_y = self.min_y;
                // self.text.push('\n');
                // self.cursor_y += 1;
                // self.cursor_x = 1;
            }
            KeyCode::Char(c) => {
                self.cursor_x += 1;
                self.text.push(c);
            }
            _ => return (false, vec![]),
        }

        (true, requests)
    }

    pub fn next_quick_select(
        &mut self,
        _code: KeyCode,
        state: &mut AppState,
    ) -> (bool, Vec<StateChangeRequest>) {
        match state.input_request().and_then(|r| r.completer()) {
            None => (),
            Some(completer) => {
                let option_count = completer.get_options(self.text.as_str()).len();

                self.quick_select += 1;
                if self.quick_select >= option_count {
                    self.quick_select = 0;
                }
            }
        }

        (false, vec![])
    }

    pub fn previous_quick_select(
        &mut self,
        _code: KeyCode,
        state: &mut AppState,
    ) -> (bool, Vec<StateChangeRequest>) {
        match state.input_request().and_then(|r| r.completer()) {
            None => (),
            Some(completer) => {
                let option_count = completer.get_options(self.text.as_str()).len();

                self.quick_select = if self.quick_select == 0 {
                    option_count - 1
                } else {
                    self.quick_select - 1
                }
            }
        }

        (false, vec![])
    }

    pub fn fill_quick_select(
        &mut self,
        code: KeyCode,
        state: &mut AppState,
    ) -> (bool, Vec<StateChangeRequest>) {
        match state.input_request().and_then(|r| r.completer()) {
            None => (),
            Some(completer) => {
                let options = completer.get_options(self.text.as_str());
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
                        self.text.extend(selection.remaining().chars());
                        self.cursor_x += selection.remaining().len().try_into().unwrap_or(u16::MAX);
                    },
                    None => return (false, vec![]),
                }
            }
        }

        (false, vec![])
    }

    pub fn fill_current_quick_select(
        &mut self,
        _code: KeyCode,
        state: &mut AppState,
    ) -> (bool, Vec<StateChangeRequest>) {
        match state.input_request().and_then(|r| r.completer()) {
            None => (),
            Some(completer) => {
                let options = completer.get_options(self.text.as_str());
                match options.get(self.quick_select) {
                    // reset quick select to start
                    None => self.quick_select = 0,
                    Some(selection) => {
                        self.text.extend(selection.remaining().chars());
                        self.cursor_x += selection.remaining().len().try_into().unwrap_or(u16::MAX);
                    }
                }
            }
        }

        (false, vec![])
    }
}

impl Panel for InputPanel {
    fn panel_type(&self) -> &str {
        INPUT_PANEL_TYPE_ID
    }

    fn init(&mut self, state: &mut AppState) {
        match make_commands() {
            Ok(commands) => self.commands = commands,
            Err(e) => state.add_error(e),
        }
    }

    fn make_widget(
        &self,
        state: &AppState,
        frame: &mut EditorFrame,
        rect: Rect,
        _is_active: bool,
        block: Block,
    ) {
        let inner_block = block.inner(rect);

        let para_text = Span::from(self.text.clone());
        let para = Paragraph::new(para_text)
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .alignment(Alignment::Left);

        let divider = Paragraph::new(Span::from("-".repeat(inner_block.width as usize)))
            .alignment(Alignment::Center);

        let (complete_text, has_completer) =
            match state.input_request().and_then(|r| r.completer()) {
                Some(completer) => (
                    completer
                        .get_options(self.text.as_str())
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
                                        .bg(match self.quick_select == i {
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
                ),
                None => (vec![], false),
            };

        let complete_para = Paragraph::new(Spans::from(complete_text))
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .alignment(Alignment::Left);

        let paras = if has_completer {
            vec![para, divider, complete_para]
        } else {
            vec![para]
        };

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                paras
                    .iter()
                    .map(|_| Constraint::Length(1))
                    .collect::<Vec<Constraint>>(),
            )
            .split(inner_block);

        frame.render_widget(block, rect);
        for (i, p) in paras.iter().enumerate() {
            frame.render_widget(p.clone(), layout[i])
        }
    }

    fn get_cursor(&self) -> (u16, u16) {
        (self.cursor_x, self.cursor_y)
    }

    fn make_title(&self, state: &AppState) -> Vec<Span> {
        match state.input_request() {
            Some(request) => {
                vec![Span::raw(request.prompt().clone())]
            }
            None => vec![],
        }
    }

    fn get_length(&self, state: &AppState) -> u16 {
        match state.input_request() {
            Some(request) => match request.completer() {
                Some(_) => 5,
                None => 3,
            },
            None => 3,
        }
    }

    fn receive_key(
        &mut self,
        event: KeyEvent,
        state: &mut AppState,
    ) -> (bool, Vec<StateChangeRequest>) {
        let (end, action) = self
            .commands
            .advance(CommandKeyId::new(event.code, event.modifiers));

        if end {
            self.commands.reset();
        }



        match action {
            Some(a) => a(self, event.code, state),
            None => (!end, vec![]),
        }
    }

    fn show(&mut self) {
        self.visible = true;
    }

    fn hide(&mut self) {
        self.visible = false;
    }

    fn visible(&self) -> bool {
        self.visible
    }
}

type InputCommand = fn(&mut InputPanel, KeyCode, &mut AppState) -> (bool, Vec<StateChangeRequest>);

pub fn make_commands() -> Result<Commands<InputCommand>, String> {
    let mut commands = Commands::<InputCommand>::new();

    commands.insert(|b| {
        b.node(catch_all())
            .action(CommandDetails::empty(), InputPanel::handle_key_stroke)
    })?;

    commands.insert(|b| {
        b.node(shift_catch_all())
            .action(CommandDetails::empty(), InputPanel::handle_key_stroke)
    })?;

    commands.insert(|b| {
        b.node(alt_catch_all())
            .action(CommandDetails::empty(), InputPanel::fill_quick_select)
    })?;

    commands.insert(|b| {
        b.node(code(KeyCode::Tab)).action(
            CommandDetails::empty(),
            InputPanel::fill_current_quick_select,
        )
    })?;

    commands.insert(|b| {
        b.node(code(KeyCode::Char('=')).mods(KeyModifiers::ALT))
            .action(CommandDetails::empty(), InputPanel::next_quick_select)
    })?;

    commands.insert(|b| {
        b.node(code(KeyCode::Char('-')).mods(KeyModifiers::ALT))
            .action(CommandDetails::empty(), InputPanel::previous_quick_select)
    })?;

    Ok(commands)
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyCode;

    use crate::app::StateChangeRequest;
    use crate::autocomplete::AutoCompleter;
    use crate::{AppState, InputPanel, Panels};

    pub struct TestCompleter {}

    impl AutoCompleter for TestCompleter {
        fn get_options(&self, s: &str) -> Vec<String> {
            ["shout", "shells", "sell", "cats", "capture"]
                .iter()
                .filter(|o| o.starts_with(s))
                .map(|s| s.to_string())
                .collect()
        }
    }

    #[test]
    fn next_quick_select() {
        let mut panels = Panels::new();
        let mut state = AppState::new();
        state.init(&mut panels);
        state.handle_changes(
            vec![StateChangeRequest::Input(
                "Test".to_string(),
                Some(Box::new(TestCompleter {})),
            )],
            &mut panels,
        );

        let mut input = InputPanel::new();

        input.next_quick_select(KeyCode::Null, &mut state);

        assert_eq!(input.quick_select, 1);
    }

    #[test]
    fn next_quick_select_past_options() {
        let mut panels = Panels::new();
        let mut state = AppState::new();
        state.init(&mut panels);
        state.handle_changes(
            vec![StateChangeRequest::Input(
                "Test".to_string(),
                Some(Box::new(TestCompleter {})),
            )],
            &mut panels,
        );

        let mut input = InputPanel::new();
        input.quick_select = 4;

        input.next_quick_select(KeyCode::Null, &mut state);

        assert_eq!(input.quick_select, 0);
    }

    #[test]
    fn previous_quick_select() {
        let mut panels = Panels::new();
        let mut state = AppState::new();
        state.init(&mut panels);
        state.handle_changes(
            vec![StateChangeRequest::Input(
                "Test".to_string(),
                Some(Box::new(TestCompleter {})),
            )],
            &mut panels,
        );

        let mut input = InputPanel::new();
        input.quick_select = 3;

        input.previous_quick_select(KeyCode::Null, &mut state);

        assert_eq!(input.quick_select, 2);
    }

    #[test]
    fn previous_quick_select_past_options() {
        let mut panels = Panels::new();
        let mut state = AppState::new();
        state.init(&mut panels);
        state.handle_changes(
            vec![StateChangeRequest::Input(
                "Test".to_string(),
                Some(Box::new(TestCompleter {})),
            )],
            &mut panels,
        );

        let mut input = InputPanel::new();

        input.previous_quick_select(KeyCode::Null, &mut state);

        assert_eq!(input.quick_select, 4);
    }

    #[test]
    fn fill_quick_select() {
        let mut panels = Panels::new();
        let mut state = AppState::new();
        state.init(&mut panels);
        state.handle_changes(
            vec![StateChangeRequest::Input(
                "Test".to_string(),
                Some(Box::new(TestCompleter {})),
            )],
            &mut panels,
        );

        let mut input = InputPanel::new();
        input.text = "se".to_string();

        input.fill_quick_select(KeyCode::Char('1'), &mut state);

        assert_eq!(input.text, "sell".to_string());
    }

    #[test]
    fn fill_quick_select_invalid_selection() {
        let mut panels = Panels::new();
        let mut state = AppState::new();
        state.init(&mut panels);
        state.handle_changes(
            vec![StateChangeRequest::Input(
                "Test".to_string(),
                Some(Box::new(TestCompleter {})),
            )],
            &mut panels,
        );

        let mut input = InputPanel::new();
        input.text = "se".to_string();

        input.fill_quick_select(KeyCode::Char('0'), &mut state);

        assert_eq!(input.text, "se".to_string());
    }

    #[test]
    fn fill_quick_select_invalid_code() {
        let mut panels = Panels::new();
        let mut state = AppState::new();
        state.init(&mut panels);
        state.handle_changes(
            vec![StateChangeRequest::Input(
                "Test".to_string(),
                Some(Box::new(TestCompleter {})),
            )],
            &mut panels,
        );

        let mut input = InputPanel::new();
        input.text = "se".to_string();

        input.fill_quick_select(KeyCode::Enter, &mut state);

        assert_eq!(input.text, "se".to_string());
    }

    #[test]
    fn fill_quick_select_out_of_option_range() {
        let mut panels = Panels::new();
        let mut state = AppState::new();
        state.init(&mut panels);
        state.handle_changes(
            vec![StateChangeRequest::Input(
                "Test".to_string(),
                Some(Box::new(TestCompleter {})),
            )],
            &mut panels,
        );

        let mut input = InputPanel::new();
        input.text = "se".to_string();

        input.fill_quick_select(KeyCode::Char('9'), &mut state);

        assert_eq!(input.text, "se".to_string());
    }

    #[test]
    fn fill_current_quick_select() {
        let mut panels = Panels::new();
        let mut state = AppState::new();
        state.init(&mut panels);
        state.handle_changes(
            vec![StateChangeRequest::Input(
                "Test".to_string(),
                Some(Box::new(TestCompleter {})),
            )],
            &mut panels,
        );

        let mut input = InputPanel::new();
        input.text = "ca".to_string();
        input.quick_select = 1;

        input.fill_current_quick_select(KeyCode::Null, &mut state);

        assert_eq!(input.text, "capture".to_string());
    }

    #[test]
    fn fill_current_quick_select_out_of_range() {
        let mut panels = Panels::new();
        let mut state = AppState::new();
        state.init(&mut panels);
        state.handle_changes(
            vec![StateChangeRequest::Input(
                "Test".to_string(),
                Some(Box::new(TestCompleter {})),
            )],
            &mut panels,
        );

        let mut input = InputPanel::new();
        input.text = "ca".to_string();
        input.quick_select = 9;

        input.fill_current_quick_select(KeyCode::Null, &mut state);

        assert_eq!(input.text, "ca".to_string());
        assert_eq!(input.quick_select, 0);
    }
}
