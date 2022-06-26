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
    CURSOR_MAX,
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
    scroll_y: u16
}

#[allow(dead_code)]
impl TextEditPanel {
    pub fn new() -> Self {
        TextEditPanel {
            scroll_y: 0,
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

    fn scroll_down(&mut self) {
        if self.scroll_y == u16::MAX {
            return;
        }
        self.scroll_y += 1;
    }

    fn scroll_up(&mut self) {
        if self.scroll_y == 0 {
            return;
        }
        self.scroll_y -= 1;
    }

    fn make_text_content(&self, text_content_box: Rect) -> (Vec<Spans>, (u16, u16), Vec<Spans>) {
        let max_text_length = text_content_box.width as usize;

        let (mut cursor_x, mut cursor_y) = CURSOR_MAX;

        let mut lines = vec![];
        let mut gutter = vec![];
        let mut real_line_count = 0;

        let mut line_start_index = 0;
        let mut line_iter = self.text.lines();

        // skip lines we have scrolled
        for _ in 0..self.scroll_y {
            match line_iter.next() {
                None => break,
                Some(text_line) => {
                    // add to line start, +1 for newline character
                    real_line_count += 1;
                    line_start_index += text_line.len() + 1;
                }
            }
        }

        // make lines, maxed to current height
        for _ in 0..text_content_box.height as usize {
            match line_iter.next() {
                None => (),
                Some(text_line) => {
                    real_line_count += 1;
                    // add 1 to account for newline character
                    let true_len = text_line.len() + 1;

                    // lines.push(Spans::from(format!("{}, {} - {} - {}", true_len, max_text_length, line_start_index, self.cursor_index)));
                    if text_line.len() < max_text_length {
                        lines.push(Spans::from(text_line));
                        gutter.push(Spans::from(Span::from(real_line_count.to_string())));

                        // lines.push(Spans::from(format!("{}", (line_start_index..(line_start_index + text_line.len())).contains(&self.cursor_index))));

                        // plus 1 to include 1 past a newline character
                        if (line_start_index..(line_start_index + true_len + 1))
                            .contains(&self.cursor_index)
                        {
                            cursor_x = text_content_box.x + (self.cursor_index - line_start_index) as u16;
                            cursor_y = text_content_box.y + lines.len() as u16 - 1;
                        }

                        line_start_index += true_len;
                    } else {
                        let (mut current, mut next) = text_line.split_at(max_text_length);
                        let continuation_length = max_text_length - self.continuation_marker.len();

                        lines.push(Spans::from(Span::from(current)));
                        gutter.push(Spans::from(Span::from(real_line_count.to_string())));

                        if (line_start_index..(line_start_index + current.len() + 1))
                            .contains(&self.cursor_index)
                        {
                            cursor_x = text_content_box.x + (self.cursor_index - line_start_index) as u16;
                            cursor_y = text_content_box.y + lines.len() as u16 - 1;
                        }

                        line_start_index += current.len();

                        while next.len() >= continuation_length {
                            (current, next) = next.split_at(continuation_length);

                            lines.push(Spans::from(vec![
                                Span::from(self.continuation_marker.as_str()),
                                Span::from(current),
                            ]));
                            gutter.push(Spans::from(Span::from(".")));

                            if (line_start_index..(line_start_index + current.len() + 1))
                                .contains(&self.cursor_index)
                            {
                                cursor_x = text_content_box.x
                                    + (self.cursor_index - line_start_index
                                    + self.continuation_marker.len())
                                    as u16;
                                cursor_y = text_content_box.y + lines.len() as u16 - 1;
                            }

                            line_start_index += current.len();
                        }

                        lines.push(Spans::from(vec![
                            Span::from(self.continuation_marker.as_str()),
                            Span::from(next),
                        ]));
                        gutter.push(Spans::from(Span::from(".")));

                        // plus 1 to include 1 past text length
                        if (line_start_index..(line_start_index + next.len() + 1))
                            .contains(&self.cursor_index)
                        {
                            cursor_x = text_content_box.x
                                + (self.cursor_index - line_start_index + self.continuation_marker.len())
                                as u16;
                            cursor_y = text_content_box.y + lines.len() as u16 - 1;
                        }

                        // plus 1 to include newline character
                        line_start_index += next.len() + 1;
                    }

                    if self.text.chars().nth(self.cursor_index - 1).unwrap() == '\n' {
                        cursor_x = text_content_box.x;
                        cursor_y = text_content_box.y + lines.len() as u16;
                    }
                }
            }
        }

        if self.text.ends_with('\n') {
            // add additional row to numbers to indicate new line has been started
            gutter.push(Spans::from(Span::from((real_line_count + 1).to_string())));
        }

        (lines, (cursor_x, cursor_y), gutter)
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

            let (lines, cursor, gutter) = self.make_text_content(layout[2]);

            let para_text = Text::from(lines);

            let line_numbers_para =
                Paragraph::new(Text::from(gutter)).alignment(Alignment::Right);

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
    use tui::text::{Span, Spans};

    use crate::{TextEditPanel};

    #[test]
    fn cursor_is_one_past_end() {
        let mut edit = TextEditPanel::new();
        edit.text = "123456789\n123456".to_string();
        edit.set_cursor_to_end();

        let (_, cursor, _) = edit.make_text_content(Rect::new(10, 10, 20, 20));

        assert_eq!(cursor, (16, 11));
    }

    #[test]
    fn cursor_is_next_line_when_after_newline() {
        let mut edit = TextEditPanel::new();
        edit.text = "123456789\n123456\n".to_string();
        edit.set_cursor_to_end();

        let (_, cursor, _) = edit.make_text_content(Rect::new(10, 10, 20, 20));

        assert_eq!(cursor, (10, 12));
    }

    #[test]
    fn cursor_on_continuation_line() {
        let mut edit = TextEditPanel::new();
        edit.text = "123456789012345678901234567890".to_string();
        edit.cursor_index = 25;

        let (_, cursor, _) = edit.make_text_content(Rect::new(10, 10, 20, 20));

        assert_eq!(
            cursor,
            (
                edit.cursor_index as u16 - 10 + edit.continuation_marker.len() as u16,
                11
            )
        );
    }

    #[test]
    fn cursor_end_of_continuation_line() {
        let mut edit = TextEditPanel::new();
        edit.text = "123456789012345678901234567890".to_string();
        edit.set_cursor_to_end();

        let (_, cursor, _) = edit.make_text_content(Rect::new(10, 10, 20, 20));

        assert_eq!(
            cursor,
            (
                edit.cursor_index as u16 - 10 + edit.continuation_marker.len() as u16,
                11
            )
        );
    }

    #[test]
    fn cursor_end_of_multiple_continuation_line() {
        let mut edit = TextEditPanel::new();
        //           |                   |               |
        edit.text = "12345678901234567890123456789012345678901234567890".to_string();
        edit.set_cursor_to_end();

        let (_, cursor, _) = edit.make_text_content(Rect::new(10, 10, 20, 20));

        assert_eq!(cursor, (24 + edit.continuation_marker.len() as u16, 12));
    }

    #[test]
    fn line_after_line_with_continuations() {
        let mut edit = TextEditPanel::new();
        //           |                   |               |
        edit.text = "12345678901234567890123456789012345678901234567890\n1234567890".to_string();
        edit.set_cursor_to_end();

        let (_, cursor, gutter) = edit.make_text_content(Rect::new(10, 10, 20, 20));

        assert_eq!(cursor, (20, 13));
        assert_eq!(gutter, vec![
            Spans::from(Span::from("1")),
            Spans::from(Span::from(".")),
            Spans::from(Span::from(".")),
            Spans::from(Span::from("2")),
        ]);
    }

    #[test]
    fn newline_after_line_with_continuations() {
        let mut edit = TextEditPanel::new();
        //           |                   |               |
        edit.text = "12345678901234567890123456789012345678901234567890\n".to_string();
        edit.set_cursor_to_end();

        let (_, cursor, gutter) = edit.make_text_content(Rect::new(10, 10, 20, 20));

        assert_eq!(cursor, (10, 13));
        assert_eq!(gutter, vec![
            Spans::from(Span::from("1")),
            Spans::from(Span::from(".")),
            Spans::from(Span::from(".")),
            Spans::from(Span::from("2")),
        ]);
    }

    #[test]
    fn lines_with_scroll() {
        let mut edit = TextEditPanel::new();
        edit.text = (100..200).map(|i| i.to_string()).collect::<Vec<String>>().join("\n");
        edit.cursor_index = 49;
        edit.scroll_y = 10;

        let (spans, cursor, gutter) = edit.make_text_content(Rect::new(10, 10, 20, 10));

        assert_eq!(cursor, (11, 12));

        assert_eq!(spans, vec![
            Spans::from(Span::from("110")),
            Spans::from(Span::from("111")),
            Spans::from(Span::from("112")),
            Spans::from(Span::from("113")),
            Spans::from(Span::from("114")),
            Spans::from(Span::from("115")),
            Spans::from(Span::from("116")),
            Spans::from(Span::from("117")),
            Spans::from(Span::from("118")),
            Spans::from(Span::from("119")),
        ]);

        assert_eq!(gutter, vec![
            Spans::from(Span::from("11")),
            Spans::from(Span::from("12")),
            Spans::from(Span::from("13")),
            Spans::from(Span::from("14")),
            Spans::from(Span::from("15")),
            Spans::from(Span::from("16")),
            Spans::from(Span::from("17")),
            Spans::from(Span::from("18")),
            Spans::from(Span::from("19")),
            Spans::from(Span::from("20")),
        ]);
    }

    #[test]
    fn scroll_down_one() {
        let mut edit = TextEditPanel::new();

        edit.scroll_down();

        assert_eq!(edit.scroll_y, 1);
    }

    #[test]
    fn scroll_down_one_at_max() {
        let mut edit = TextEditPanel::new();
        edit.scroll_y = u16::MAX;

        edit.scroll_down();

        assert_eq!(edit.scroll_y, u16::MAX);
    }

    #[test]
    fn scroll_up_one() {
        let mut edit = TextEditPanel::new();
        edit.scroll_y = 6;

        edit.scroll_up();

        assert_eq!(edit.scroll_y, 5);
    }

    #[test]
    fn scroll_up_one_at_zero() {
        let mut edit = TextEditPanel::new();

        edit.scroll_up();

        assert_eq!(edit.scroll_y, 0);
    }
}
