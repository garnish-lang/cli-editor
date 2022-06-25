use std::env;
use std::fs;
use std::io::Read;
use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent};
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Style};
use tui::text::{Span, Spans, Text};
use tui::widgets::{Block, Paragraph};

use crate::app::StateChangeRequest;
use crate::autocomplete::FileAutoCompleter;
use crate::commands::shift_catch_all;
use crate::panels::RenderDetails;
use crate::{
    catch_all, ctrl_key, AppState, CommandDetails, CommandKeyId, Commands, EditorFrame, Panel,
};

pub const EDIT_PANEL_TYPE_ID: &str = "Edit";

pub struct TextEditPanel {
    cursor_index: usize,
    text: String,
    title: String,
    commands: Commands<EditCommand>,
    file_path: PathBuf,
    gutter_size: u16,
    continuation_marker: String,
}

#[allow(dead_code)]
impl TextEditPanel {
    pub fn new() -> Self {
        TextEditPanel {
            cursor_index: 0,
            gutter_size: 5,
            text: String::new(),
            title: "Buffer".to_string(),
            commands: Commands::<EditCommand>::new(),
            file_path: PathBuf::new(),
            continuation_marker: "... ".to_string(),
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
            KeyCode::Backspace => match self.text.pop() {
                None => {
                    self.cursor_index = 0;
                }
                Some(_) => {
                    self.cursor_index -= 1;
                }
            },
            KeyCode::Delete => {
                // ??
            }
            KeyCode::Enter => {
                self.text.push('\n');
                self.cursor_index += 1;
            }
            KeyCode::Char(c) => {
                self.text.push(c);
                self.cursor_index += 1;
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
        self.cursor_index = self.text.len();
    }

    fn make_text_content(&self, text_content_box: Rect) -> (Vec<Spans>, (u16, u16)) {
        let max_text_length = text_content_box.width as usize;
        let continuation_length =
            max_text_length - self.continuation_marker.len();

        let (mut cursor_x, mut cursor_y) = (text_content_box.x, text_content_box.y);

        let mut lines = vec![];

        let mut line_start_index = 0;
        for text_line in self.text.lines() {
            // add 1 to account for newline character
            let true_len = text_line.len() + 1;

            // lines.push(Spans::from(format!("{}, {} - {} - {}", true_len, max_text_length, line_start_index, self.cursor_index)));
            if text_line.len() < max_text_length {
                lines.push(Spans::from(text_line));

                // lines.push(Spans::from(format!("{}", (line_start_index..(line_start_index + text_line.len())).contains(&self.cursor_index))));

                // plus 1 to include 1 past a newline character
                if (line_start_index..(line_start_index + true_len + 1)).contains(&self.cursor_index) {
                    if self.text.chars().nth(self.cursor_index - 1).unwrap() == '\n' {
                        cursor_x = text_content_box.x;
                        cursor_y += lines.len() as u16;
                    } else {
                        cursor_x += (self.cursor_index - line_start_index) as u16;
                        cursor_y += lines.len() as u16 - 1;
                    }
                }

                line_start_index += true_len;
            } else {
                let (mut current, mut next) = text_line.split_at(max_text_length);
                lines.push(Spans::from(Span::from(current)));

                while next.len() >= continuation_length {
                    if (line_start_index..(line_start_index + current.len())).contains(&self.cursor_index) {
                        cursor_x += (self.cursor_index - line_start_index
                            + self.continuation_marker.len())
                            as u16;
                        cursor_y += (lines.len()) as u16;
                    }

                    (current, next) = next.split_at(continuation_length);

                    line_start_index += current.len();

                    lines.push(Spans::from(vec![
                        Span::from(self.continuation_marker.as_str()),
                        Span::from(current),
                    ]));
                }

                line_start_index += current.len();

                if (line_start_index..(line_start_index + current.len())).contains(&self.cursor_index) {
                    cursor_x += (self.cursor_index - line_start_index
                        + self.continuation_marker.len()) as u16;
                    cursor_y += (lines.len()) as u16;
                }

                lines.push(Spans::from(vec![
                    Span::from(self.continuation_marker.as_str()),
                    Span::from(next),
                ]));
            }
        }

        (lines, (cursor_x, cursor_y))
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
    ) -> RenderDetails {
        if !self.text.is_empty() {
            let line_count = self.text.lines().count();
            let line_count_size = line_count.to_string().len().min(u16::MAX as usize) as u16;

            let layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(vec![
                    Constraint::Length(line_count_size),
                    Constraint::Length(self.gutter_size),
                    Constraint::Length(rect.width - line_count_size - self.gutter_size),
                ])
                .split(rect);

            let gutter_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(vec![
                    Constraint::Length(1),
                    Constraint::Length(self.gutter_size - 2),
                    Constraint::Length(1),
                ])
                .split(layout[1]);

            let (lines, cursor) = self.make_text_content(layout[2]);

            let para_text = Text::from(lines);

            let line_numbers = (1..rect.height + 1)
                .map(|i| i.to_string())
                .collect::<Vec<String>>()
                .join("\n");

            let line_numbers_para =
                Paragraph::new(Text::from(line_numbers)).alignment(Alignment::Right);

            frame.render_widget(line_numbers_para, layout[0]);

            let gutter = Block::default().style(Style::default().bg(Color::DarkGray));

            frame.render_widget(gutter, gutter_layout[1]);

            let para =
                Paragraph::new(para_text).style(Style::default().fg(Color::White).bg(Color::Black));

            frame.render_widget(para, layout[2]);

            RenderDetails::new(vec![Span::raw(self.title.clone())], cursor)
        } else {
            RenderDetails::new(vec![Span::raw(self.title.clone())], (1, 1))
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

#[cfg(test)]
mod tests {
    use tui::layout::Rect;
    use crate::{AppState, TextEditPanel};

    #[test]
    fn cursor_is_one_past_end() {
        let mut edit = TextEditPanel::new();
        edit.text = "123456789\n123456".to_string();
        edit.cursor_index = edit.text.len();

        let (spans, cursor) = edit.make_text_content(Rect::new(10, 10, 20, 20));

        assert_eq!(cursor, (16, 11));
    }

    #[test]
    fn cursor_is_next_line_when_after_newline() {
        let mut edit = TextEditPanel::new();
        edit.text = "123456789\n123456\n".to_string();
        edit.cursor_index = edit.text.len();

        let (spans, cursor) = edit.make_text_content(Rect::new(10, 10, 20, 20));

        assert_eq!(cursor, (10, 12));
    }
}