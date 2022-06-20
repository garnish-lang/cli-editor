use crossterm::event::{KeyCode, KeyEvent};
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Style};
use tui::text::Span;
use tui::widgets::{Block, Paragraph};

use crate::{AppState, catch_all, CommandDetails, CommandKeyId, Commands, EditorFrame, Panel};
use crate::app::StateChangeRequest;
use crate::commands::shift_catch_all;

pub const INPUT_PANEL_TYPE_ID: &str = "Input";

pub struct InputPanel {
    min_x: u16,
    min_y: u16,
    cursor_x: u16,
    cursor_y: u16,
    text: String,
    commands: Commands<InputCommand>,
    visible: bool,
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
        }
    }

    #[allow(dead_code)]
    pub fn set_text(&mut self, text: String) {
        self.text = text;
    }

    fn handle_key_stroke(&mut self, code: KeyCode) -> (bool, Vec<StateChangeRequest>) {
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

        let (complete_text, has_completer) = match state.input_request().and_then(|r| r.completer()) {
            Some(completer) => (completer.get_options(self.text.as_str()).join(" | "), true),
            None => (String::new(), false),
        };

        let complete_para = Paragraph::new(complete_text)
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .alignment(Alignment::Left);

        let paras = if has_completer {
            vec![
                para,
                divider,
                complete_para
            ]
        } else {
            vec![para]
        };

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(paras.iter().map(|_| Constraint::Length(1)).collect::<Vec<Constraint>>())
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

    fn receive_key(&mut self, event: KeyEvent) -> (bool, Vec<StateChangeRequest>) {
        let (end, action) = self
            .commands
            .advance(CommandKeyId::new(event.code, event.modifiers));

        if end {
            self.commands.reset();
        }

        match action {
            Some(a) => a(self, event.code),
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

type InputCommand = fn(&mut InputPanel, KeyCode) -> (bool, Vec<StateChangeRequest>);

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

    Ok(commands)
}
