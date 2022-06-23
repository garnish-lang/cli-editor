use std::env;
use std::fs;
use std::io::Read;
use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent};
use tui::layout::{Alignment, Rect};
use tui::style::{Color, Style};
use tui::text::{Span, Text};
use tui::widgets::{Block, Paragraph, Wrap};

use crate::app::StateChangeRequest;
use crate::autocomplete::FileAutoCompleter;
use crate::commands::shift_catch_all;
use crate::{
    catch_all, ctrl_key, AppState, CommandDetails, CommandKeyId, Commands, EditorFrame, Panel,
};
use crate::panels::RenderDetails;

pub const EDIT_PANEL_TYPE_ID: &str = "Edit";

pub struct TextEditPanel {
    min_x: u16,
    min_y: u16,
    cursor_x: u16,
    cursor_y: u16,
    text: String,
    title: String,
    commands: Commands<EditCommand>,
    file_path: PathBuf,
}

#[allow(dead_code)]
impl TextEditPanel {
    pub fn new() -> Self {
        TextEditPanel {
            cursor_x: 1,
            cursor_y: 1,
            min_x: 1,
            min_y: 1,
            text: String::new(),
            title: "Buffer".to_string(),
            commands: Commands::<EditCommand>::new(),
            file_path: PathBuf::new(),
        }
    }

    pub fn get_text(&self) -> &str {
        &self.text
    }

    pub fn set_text(&mut self, text: String) {
        self.text = text;
    }

    fn handle_key_stroke(
        &mut self,
        code: KeyCode,
        _state: &mut AppState,
    ) -> (bool, Vec<StateChangeRequest>) {
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
                self.text.push('\n');
                self.cursor_y += 1;
                self.cursor_x = 1;
            }
            KeyCode::Char(c) => {
                self.cursor_x += 1;
                self.text.push(c);
            }
            _ => return (false, vec![]),
        }

        (true, vec![])
    }

    fn open_file(
        &mut self,
        _code: KeyCode,
        state: &mut AppState,
    ) -> (bool, Vec<StateChangeRequest>) {
        state.add_info(format!("request open file"));
        (
            true,
            vec![StateChangeRequest::input_request_with_completer(
                "File Name".to_string(),
                Box::new(FileAutoCompleter::new()),
            )],
        )
    }

    fn set_cursor_to_end(&mut self) {
        self.cursor_x = self.min_x;
        self.cursor_y = self.min_y;

        for c in self.text.chars() {
            match c {
                '\n' => {
                    self.cursor_x = self.min_x;
                    self.cursor_y += 1;
                }
                _ => {
                    self.cursor_x += 1;
                }
            }
        }
    }
}

impl Panel for TextEditPanel {
    fn panel_type(&self) -> &str {
        EDIT_PANEL_TYPE_ID
    }

    fn init(&mut self, state: &mut AppState) {
        match make_commands() {
            Ok(commands) => self.commands = commands,
            Err(e) => state.add_error(e),
        }
    }

    fn make_widget(
        &self,
        _state: &AppState,
        frame: &mut EditorFrame,
        rect: Rect,
        _is_active: bool,
        block: Block,
    ) -> RenderDetails {
        let para_text = Text::from(self.text.clone());
        let para = Paragraph::new(para_text)
            .block(block)
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true });

        frame.render_widget(para, rect);

        RenderDetails::new(vec![], (self.cursor_x, self.cursor_y))
    }

    fn get_cursor(&self) -> (u16, u16) {
        (self.cursor_x, self.cursor_y)
    }

    fn make_title(&self, _state: &AppState) -> Vec<Span> {
        vec![Span::raw(self.title.clone())]
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

    fn receive_input(&mut self, input: String) -> Vec<StateChangeRequest> {
        let mut changes = vec![];

        let current_dir = match env::current_dir() {
            Err(e) => {
                changes.push(StateChangeRequest::error(e));
                return changes;
            }
            Ok(p) => p,
        };

        self.file_path = (&current_dir).clone();
        self.file_path.push(input);

        match fs::File::open(&self.file_path) {
            Err(e) => changes.push(StateChangeRequest::error(e)),
            Ok(mut file) => {
                let mut s = String::new();
                match file.read_to_string(&mut s) {
                    Err(e) => changes.push(StateChangeRequest::error(e)),
                    Ok(_) => {
                        self.text = s;
                        self.title = if self.file_path.starts_with(&current_dir) {
                            match self.file_path.strip_prefix(&current_dir) {
                                Err(e) => {
                                    changes.push(StateChangeRequest::error(e));
                                    self.file_path.to_string_lossy().to_string()
                                }
                                Ok(p) => p.as_os_str().to_string_lossy().to_string(),
                            }
                        } else {
                            self.file_path.to_string_lossy().to_string()
                        }
                    }
                }
            }
        };

        self.set_cursor_to_end();

        changes
    }
}

type EditCommand =
    fn(&mut TextEditPanel, KeyCode, &mut AppState) -> (bool, Vec<StateChangeRequest>);

pub fn make_commands() -> Result<Commands<EditCommand>, String> {
    let mut commands = Commands::<EditCommand>::new();

    commands.insert(|b| {
        b.node(catch_all())
            .action(CommandDetails::empty(), TextEditPanel::handle_key_stroke)
    })?;

    commands.insert(|b| {
        b.node(shift_catch_all())
            .action(CommandDetails::empty(), TextEditPanel::handle_key_stroke)
    })?;

    commands.insert(|b| {
        b.node(ctrl_key('o'))
            .action(CommandDetails::open_file(), TextEditPanel::open_file)
    })?;

    Ok(commands)
}
