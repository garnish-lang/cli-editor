use std::cmp::min;
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
use crate::commands::{alt_key, shift_alt_key, shift_catch_all};
use crate::panels::RenderDetails;
use crate::{
    catch_all, ctrl_key, AppState, CommandDetails, CommandKeyId, Commands, EditorFrame, Panel,
    CURSOR_MAX,
};

pub const EDIT_PANEL_TYPE_ID: &str = "Edit";

pub struct TextEditPanel {
    cursor_index: usize,
    current_line: usize,
    cursor_index_in_line: usize,
    text: String,
    title: String,
    commands: Commands<EditCommand>,
    file_path: PathBuf,
    gutter_size: u16,
    continuation_marker: String,
    scroll_y: u16,
    lines: Vec<String>
}

#[allow(dead_code)]
impl TextEditPanel {
    pub fn new() -> Self {
        TextEditPanel {
            scroll_y: 0,
            cursor_index: 0,
            current_line: 0,
            cursor_index_in_line: 0,
            gutter_size: 5,
            text: String::new(),
            title: "Buffer".to_string(),
            commands: Commands::<EditCommand>::new(),
            file_path: PathBuf::new(),
            continuation_marker: "... ".to_string(),
            lines: vec![]
        }
    }

    pub fn get_text(&self) -> &str {
        &self.text
    }

    pub fn set_text<T: ToString>(&mut self, text: T) {
        self.text = text.to_string();
        self.lines = self.text.lines().map(|s| s.to_string()).collect();
    }

    fn remove_character(&mut self, index_adjustment: usize, movement: usize, state: &mut AppState) {
        match self.lines.get_mut(self.current_line) {
            None => (), // no text, do nothing
            Some(line) => {
                if self.cursor_index_in_line - index_adjustment < line.len() {
                    line.remove(self.cursor_index_in_line - index_adjustment);
                    self.cursor_index_in_line -= movement;
                } else {
                    // cursor isn't in line
                    // implementation error
                    // log message and reset cursor to start of line
                    self.cursor_index_in_line = 0;
                    state.add_error("Cursor outside of current line. Resetting to start of line.");
                }
            }
        }
    }

    fn remove_line(&mut self) {
        if self.current_line != 0 {
            let remaining = self.lines.remove(self.current_line);
            self.current_line -= 1;
            self.cursor_index_in_line = match self.lines.get_mut(self.current_line) {
                None => 0, // needs a test
                Some(line) => {
                    // add remaining characters to this line
                    // but cursor will be at end of existing characters
                    let existing_len = line.len();

                    line.extend(remaining.chars());

                    existing_len
                }
            }
        }
    }

    fn handle_key_stroke(
        &mut self,
        code: KeyCode,
        state: &mut AppState,
    ) -> (bool, Vec<StateChangeRequest>) {
        match code {
            KeyCode::Backspace => {
                match self.text.pop() {
                    None => {
                        self.cursor_index = 0;
                    }
                    Some(_) => {
                        self.cursor_index -= 1;
                    }
                }

                // new
                if self.cursor_index_in_line == 0 {
                    self.remove_line();
                } else {
                    self.remove_character(1, 1, state);
                }
            },
            KeyCode::Delete => {
                match self.lines.get(self.current_line) {
                    None => (),
                    Some(line) => if self.cursor_index_in_line == line.len() {
                        self.current_line += 1;
                        self.remove_line();
                    } else {
                        self.remove_character(0, 0, state);
                    }
                }
            }
            KeyCode::Enter => {
                self.text.push('\n');
                self.cursor_index += 1;

                // new
                self.lines.push(String::new());
                self.current_line += 1;
                self.cursor_index_in_line = 0;
            }
            KeyCode::Char(c) => {
                self.text.push(c);
                self.cursor_index += 1;

                // new
                match self.lines.get_mut(self.current_line) {
                    None => {
                        // start new
                        self.lines.push(c.to_string());
                    }
                    Some(s) => {
                        // add to existing
                        s.insert(self.cursor_index_in_line, c);
                    }
                }
                self.cursor_index_in_line += 1;
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

    fn set_cursor_index(&mut self, index: usize) {
        self.cursor_index = min(self.text.len(), index);
    }

    fn move_to_next_character(
        &mut self,
        _code: KeyCode,
        _state: &mut AppState,
    ) -> (bool, Vec<StateChangeRequest>) {
        self.set_cursor_index(self.cursor_index + 1);

        (true, vec![])
    }

    fn move_to_previous_character(
        &mut self,
        _code: KeyCode,
        _state: &mut AppState,
    ) -> (bool, Vec<StateChangeRequest>) {
        if self.cursor_index > 0 {
            self.set_cursor_index(self.cursor_index - 1);
        }

        (true, vec![])
    }

    fn move_to_next_line(
        &mut self,
        _code: KeyCode,
        _state: &mut AppState,
    ) -> (bool, Vec<StateChangeRequest>) {
        // split at current index
        // count back to newline for current line index
        // advance forward to find next line
        // advance again equal to current line index, or end of line
        let (front, back) = self.text.split_at(self.cursor_index);
        // split at, splits before index, putting current character in back
        let distance_from_start = match front.rfind('\n') {
            // reached start of text, distance is len of front
            None => front.len(),
            // found newline, distance is len minus index and additional 1 to exclude newline
            Some(index) => front.len() - index - 1,
        };

        match back.find('\n') {
            // reach end of text
            None => {
                self.set_cursor_to_end();
            }
            Some(index) => {
                let current_line_start = index + 1;
                // if distance would put us past this next line
                // stop at end of line
                let current_line_len = match back[current_line_start..].find('\n') {
                    // subtract index + 1, for all character that aren't a part of current line
                    None => back.len() - current_line_start,
                    // this index is relative to slice in match
                    // len will be the index itself + 1 for newline
                    Some(index) => index,
                };

                let back_index = min(current_line_start + distance_from_start, index + current_line_len);

                let l = front.len();
                // index is relative to back, add front len for real index
                // add distance for new index
                self.set_cursor_index(l + back_index);
            }
        }

        (true, vec![])
    }

    fn move_to_previous_line(
        &mut self,
        _code: KeyCode,
        _state: &mut AppState,
    ) -> (bool, Vec<StateChangeRequest>) {
        // split at current index
        // count back to find last newline and distance from newline
        // count again from first newline to find second
        // add distance to get new index

        let (front, _) = self.text.split_at(self.cursor_index);

        let (newline_index, distance) = match front.rfind('\n') {
            // beginning of text, set to start and end
            None => {
                self.cursor_index = 0;
                return (true, vec![])
            }
            Some(index) => (index, front.len() - index)
        };

        match front[..newline_index].rfind('\n') {
            // beginning of text, set to lowest of distance and newline index, then end
            None => {
                self.set_cursor_index(min(newline_index, distance - 1));
                return (true, vec![])
            }
            Some(index) => {
                // if distance would put us past this next line
                // stop at end of line

                let current_line_len = front[..newline_index].len();

                let back_index = min(current_line_len, distance);

                self.set_cursor_index(index + back_index);
            }
        }

        (true, vec![])
    }

    fn scroll_down(&mut self, amount: u16) {
        if self.scroll_y < u16::MAX - amount {
            self.scroll_y += amount;
        } else {
            self.scroll_y = u16::MAX;
        }
    }

    fn scroll_up(&mut self, amount: u16) {
        if self.scroll_y >= amount {
            self.scroll_y -= amount;
        } else {
            self.scroll_y = 0;
        }
    }

    fn scroll_down_one(
        &mut self,
        _code: KeyCode,
        _state: &mut AppState,
    ) -> (bool, Vec<StateChangeRequest>) {
        self.scroll_down(1);
        (true, vec![])
    }

    fn scroll_up_one(
        &mut self,
        _code: KeyCode,
        _state: &mut AppState,
    ) -> (bool, Vec<StateChangeRequest>) {
        self.scroll_up(1);
        (true, vec![])
    }

    fn scroll_down_ten(
        &mut self,
        _code: KeyCode,
        _state: &mut AppState,
    ) -> (bool, Vec<StateChangeRequest>) {
        let limit = self.text.lines().count() as u16;
        self.scroll_down(10);

        if self.scroll_y > limit {
            self.scroll_y = limit;
        }

        (true, vec![])
    }

    fn scroll_up_ten(
        &mut self,
        _code: KeyCode,
        _state: &mut AppState,
    ) -> (bool, Vec<StateChangeRequest>) {
        self.scroll_up(10);
        (true, vec![])
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
                            cursor_x =
                                text_content_box.x + (self.cursor_index - line_start_index) as u16;
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
                            cursor_x =
                                text_content_box.x + (self.cursor_index - line_start_index) as u16;
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
                                + (self.cursor_index - line_start_index
                                    + self.continuation_marker.len())
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

            let line_numbers_para = Paragraph::new(Text::from(gutter)).alignment(Alignment::Right);

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
                        self.set_text(s);

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

        self.cursor_index = 0;
        self.scroll_y = 0;

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

    commands.insert(|b| {
        b.node(alt_key('i'))
            .action(CommandDetails::empty(), TextEditPanel::scroll_up_one)
    })?;

    commands.insert(|b| {
        b.node(alt_key('k'))
            .action(CommandDetails::empty(), TextEditPanel::scroll_down_one)
    })?;

    commands.insert(|b| {
        b.node(shift_alt_key('I'))
            .action(CommandDetails::empty(), TextEditPanel::scroll_up_ten)
    })?;

    commands.insert(|b| {
        b.node(shift_alt_key('K'))
            .action(CommandDetails::empty(), TextEditPanel::scroll_down_ten)
    })?;

    Ok(commands)
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyCode;
    use tui::layout::Rect;
    use tui::text::{Span, Spans};

    use crate::{AppState, TextEditPanel};

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
        assert_eq!(
            gutter,
            vec![
                Spans::from(Span::from("1")),
                Spans::from(Span::from(".")),
                Spans::from(Span::from(".")),
                Spans::from(Span::from("2")),
            ]
        );
    }

    #[test]
    fn newline_after_line_with_continuations() {
        let mut edit = TextEditPanel::new();
        //           |                   |               |
        edit.text = "12345678901234567890123456789012345678901234567890\n".to_string();
        edit.set_cursor_to_end();

        let (_, cursor, gutter) = edit.make_text_content(Rect::new(10, 10, 20, 20));

        assert_eq!(cursor, (10, 13));
        assert_eq!(
            gutter,
            vec![
                Spans::from(Span::from("1")),
                Spans::from(Span::from(".")),
                Spans::from(Span::from(".")),
                Spans::from(Span::from("2")),
            ]
        );
    }

    #[test]
    fn lines_with_scroll() {
        let mut edit = TextEditPanel::new();
        edit.text = (100..200)
            .map(|i| i.to_string())
            .collect::<Vec<String>>()
            .join("\n");
        edit.cursor_index = 49;
        edit.scroll_y = 10;

        let (spans, cursor, gutter) = edit.make_text_content(Rect::new(10, 10, 20, 10));

        assert_eq!(cursor, (11, 12));

        assert_eq!(
            spans,
            vec![
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
            ]
        );

        assert_eq!(
            gutter,
            vec![
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
            ]
        );
    }

    #[test]
    fn handle_character_key() {
        let mut edit = TextEditPanel::new();
        let mut state = AppState::new();

        edit.handle_key_stroke(KeyCode::Char('a'), &mut state);

        assert_eq!(edit.lines, vec!["a".to_string()]);
        assert_eq!(edit.cursor_index_in_line, 1);
    }

    #[test]
    fn handle_character_key_middle_of_line() {
        let mut edit = TextEditPanel::new();
        edit.set_text("ac");
        edit.cursor_index = 1;
        edit.cursor_index_in_line = 1;

        let mut state = AppState::new();

        edit.handle_key_stroke(KeyCode::Char('b'), &mut state);

        assert_eq!(edit.lines, vec!["abc".to_string()]);
        assert_eq!(edit.cursor_index_in_line, 2);
    }

    #[test]
    fn handle_character_key_with_existing_text() {
        let mut edit = TextEditPanel::new();
        edit.set_text("a");
        edit.cursor_index_in_line = 1;

        let mut state = AppState::new();

        edit.handle_key_stroke(KeyCode::Char('b'), &mut state);

        assert_eq!(edit.lines, vec!["ab".to_string()]);
        assert_eq!(edit.cursor_index_in_line, 2);
    }

    #[test]
    fn handle_enter_key() {
        let mut edit = TextEditPanel::new();
        edit.set_text("a");
        edit.cursor_index_in_line = 1;

        let mut state = AppState::new();

        edit.handle_key_stroke(KeyCode::Enter, &mut state);

        assert_eq!(edit.lines, vec!["a".to_string(), String::new()]);
        assert_eq!(edit.current_line, 1);
        assert_eq!(edit.cursor_index_in_line, 0);
    }

    #[test]
    fn handle_backspace_key() {
        let mut edit = TextEditPanel::new();
        edit.set_text("a");
        edit.cursor_index = 1;
        edit.cursor_index_in_line = 1;

        let mut state = AppState::new();

        edit.handle_key_stroke(KeyCode::Backspace, &mut state);

        assert_eq!(edit.lines, vec!["".to_string()]);
        assert_eq!(edit.current_line, 0);
        assert_eq!(edit.cursor_index_in_line, 0);
    }

    #[test]
    fn handle_backspace_key_middle_of_line() {
        let mut edit = TextEditPanel::new();
        edit.set_text("abc");
        edit.cursor_index = 1;
        edit.cursor_index_in_line = 2;

        let mut state = AppState::new();

        edit.handle_key_stroke(KeyCode::Backspace, &mut state);

        assert_eq!(edit.lines, vec!["ac".to_string()]);
        assert_eq!(edit.current_line, 0);
        assert_eq!(edit.cursor_index_in_line, 1);
    }

    #[test]
    fn handle_backspace_key_start_of_non_empty_line() {
        let mut edit = TextEditPanel::new();
        edit.set_text("abc\ndef");
        edit.cursor_index = 1;
        edit.current_line = 1;
        edit.cursor_index_in_line = 0;

        let mut state = AppState::new();

        edit.handle_key_stroke(KeyCode::Backspace, &mut state);

        assert_eq!(edit.lines, vec!["abcdef".to_string()]);
        assert_eq!(edit.current_line, 0);
        assert_eq!(edit.cursor_index_in_line, 3);
    }

    #[test]
    fn handle_backspace_key_on_newline() {
        let mut edit = TextEditPanel::new();
        edit.set_text("a\na");
        edit.cursor_index = 2;
        edit.cursor_index_in_line = 1;
        edit.current_line = 1;

        let mut state = AppState::new();

        edit.handle_key_stroke(KeyCode::Backspace, &mut state);
        edit.handle_key_stroke(KeyCode::Backspace, &mut state);

        assert_eq!(edit.lines, vec!["a".to_string()]);
        assert_eq!(edit.current_line, 0);
        assert_eq!(edit.cursor_index_in_line, 1);
    }

    #[test]
    fn handle_delete_key() {
        let mut edit = TextEditPanel::new();
        edit.set_text("a");
        edit.cursor_index = 1;
        edit.cursor_index_in_line = 0;

        let mut state = AppState::new();

        edit.handle_key_stroke(KeyCode::Delete, &mut state);

        assert_eq!(edit.lines, vec!["".to_string()]);
        assert_eq!(edit.current_line, 0);
        assert_eq!(edit.cursor_index_in_line, 0);
    }

    #[test]
    fn handle_delete_key_middle_of_line() {
        let mut edit = TextEditPanel::new();
        edit.set_text("abc");
        edit.cursor_index = 1;
        edit.cursor_index_in_line = 1;

        let mut state = AppState::new();

        edit.handle_key_stroke(KeyCode::Delete, &mut state);

        assert_eq!(edit.lines, vec!["ac".to_string()]);
        assert_eq!(edit.current_line, 0);
        assert_eq!(edit.cursor_index_in_line, 1);
    }

    #[test]
    fn handle_delete_key_on_empty_line() {
        let mut edit = TextEditPanel::new();
        edit.set_text("abc\n\ndef");
        edit.cursor_index = 1;
        edit.current_line = 1;
        edit.cursor_index_in_line = 0;

        let mut state = AppState::new();

        edit.handle_key_stroke(KeyCode::Delete, &mut state);

        assert_eq!(edit.lines, vec!["abc".to_string(), "def".to_string()]);
        assert_eq!(edit.current_line, 1);
        assert_eq!(edit.cursor_index_in_line, 0);
    }

    #[test]
    fn handle_delete_key_end_of_line() {
        let mut edit = TextEditPanel::new();
        edit.set_text("abc\ndef");
        edit.cursor_index = 1;
        edit.current_line = 0;
        edit.cursor_index_in_line = 3;

        let mut state = AppState::new();

        edit.handle_key_stroke(KeyCode::Delete, &mut state);

        assert_eq!(edit.lines, vec!["abcdef".to_string()]);
        assert_eq!(edit.current_line, 0);
        assert_eq!(edit.cursor_index_in_line, 3);
    }
    
    #[test]
    fn scroll_down() {
        let mut edit = TextEditPanel::new();
        edit.scroll_y = 5;

        edit.scroll_down(12);

        assert_eq!(edit.scroll_y, 17);
    }

    #[test]
    fn scroll_down_past_limit() {
        let mut edit = TextEditPanel::new();
        edit.scroll_y = u16::MAX - 5;

        edit.scroll_down(10);

        assert_eq!(edit.scroll_y, u16::MAX);
    }

    #[test]
    fn scroll_up() {
        let mut edit = TextEditPanel::new();
        edit.scroll_y = 5;

        edit.scroll_up(4);

        assert_eq!(edit.scroll_y, 1);
    }

    #[test]
    fn scroll_up_past_limit() {
        let mut edit = TextEditPanel::new();
        edit.scroll_y = 5;

        edit.scroll_up(10);

        assert_eq!(edit.scroll_y, 0);
    }

    #[test]
    fn scroll_down_one() {
        let mut edit = TextEditPanel::new();
        edit.set_text((100..200)
            .map(|i| i.to_string())
            .collect::<Vec<String>>()
            .join("\n"));
        edit.scroll_y = 95;

        let mut state = AppState::new();

        edit.scroll_down_one(KeyCode::Null, &mut state);

        assert_eq!(edit.scroll_y, 96);
    }

    #[test]
    fn scroll_down_past_text() {
        let mut edit = TextEditPanel::new();
        edit.set_text((100..200)
            .map(|i| i.to_string())
            .collect::<Vec<String>>()
            .join("\n"));
        edit.scroll_y = 95;

        let mut state = AppState::new();

        edit.scroll_down_ten(KeyCode::Null, &mut state);

        assert_eq!(edit.scroll_y, 100);
    }

    #[test]
    fn scroll_up_one() {
        let mut edit = TextEditPanel::new();
        let mut state = AppState::new();
        edit.scroll_y = 6;

        edit.scroll_up_one(KeyCode::Null, &mut state);

        assert_eq!(edit.scroll_y, 5);
    }

    #[test]
    fn scroll_up_one_at_zero() {
        let mut edit = TextEditPanel::new();
        let mut state = AppState::new();

        edit.scroll_up_one(KeyCode::Null, &mut state);

        assert_eq!(edit.scroll_y, 0);
    }

    #[test]
    fn set_cursor() {
        let mut edit = TextEditPanel::new();
        edit.set_text((100..200)
            .map(|i| i.to_string())
            .collect::<Vec<String>>()
            .join("\n"));

        edit.set_cursor_index(10);

        assert_eq!(edit.cursor_index, 10);
    }

    #[test]
    fn set_cursor_past_text() {
        let mut edit = TextEditPanel::new();
        edit.set_text((100..200)
            .map(|i| i.to_string())
            .collect::<Vec<String>>()
            .join("\n"));

        edit.set_cursor_index(usize::MAX);

        assert_eq!(edit.cursor_index, edit.text.len());
    }

    #[test]
    fn next_character() {
        let mut edit = TextEditPanel::new();
        edit.set_text((100..200)
            .map(|i| i.to_string())
            .collect::<Vec<String>>()
            .join("\n"));
        edit.cursor_index = 10;
        let mut state = AppState::new();

        edit.move_to_next_character(KeyCode::Null, &mut state);

        assert_eq!(edit.cursor_index, 11);
    }

    #[test]
    fn previous_character() {
        let mut edit = TextEditPanel::new();
        edit.set_text((100..200)
            .map(|i| i.to_string())
            .collect::<Vec<String>>()
            .join("\n"));
        edit.cursor_index = 10;
        let mut state = AppState::new();

        edit.move_to_previous_character(KeyCode::Null, &mut state);

        assert_eq!(edit.cursor_index, 9);
    }

    #[test]
    fn previous_character_at_zero() {
        let mut edit = TextEditPanel::new();
        edit.set_text((100..200)
            .map(|i| i.to_string())
            .collect::<Vec<String>>()
            .join("\n"));
        let mut state = AppState::new();

        edit.move_to_previous_character(KeyCode::Null, &mut state);

        assert_eq!(edit.cursor_index, 0);
    }

    #[test]
    fn next_line() {
        let mut edit = TextEditPanel::new();
        edit.set_text("12345\n12345\n".to_string());
        edit.cursor_index = 4;
        let mut state = AppState::new();

        edit.move_to_next_line(KeyCode::Null, &mut state);

        assert_eq!(edit.cursor_index, 10);
    }

    #[test]
    fn next_line_no_line() {
        let mut edit = TextEditPanel::new();
        edit.set_text("1234567890".to_string());
        edit.cursor_index = 4;
        let mut state = AppState::new();

        edit.move_to_next_line(KeyCode::Null, &mut state);

        assert_eq!(edit.cursor_index, 10);
    }

    #[test]
    fn next_line_longer_than_next() {
        let mut edit = TextEditPanel::new();
        edit.set_text("1234567890\n12345\n1234567890".to_string());
        edit.cursor_index = 9;
        let mut state = AppState::new();

        edit.move_to_next_line(KeyCode::Null, &mut state);

        assert_eq!(edit.cursor_index, 15);
    }

    #[test]
    fn next_line_that_is_last_line() {
        let mut edit = TextEditPanel::new();
        edit.set_text("1234567890\n12345\n1234567890".to_string());
        edit.cursor_index = 12;
        let mut state = AppState::new();

        edit.move_to_next_line(KeyCode::Null, &mut state);

        assert_eq!(edit.cursor_index, 18);
    }

    #[test]
    fn previous_line() {
        let mut edit = TextEditPanel::new();
        edit.set_text("1234567890\n1234567890".to_string());
        edit.cursor_index = 17;
        let mut state = AppState::new();

        edit.move_to_previous_line(KeyCode::Null, &mut state);

        assert_eq!(edit.cursor_index, 6);
    }

    #[test]
    fn next_line_longer_than_previous() {
        let mut edit = TextEditPanel::new();
        edit.set_text("12345\n1234567890".to_string());
        edit.cursor_index = edit.text.len();
        let mut state = AppState::new();

        edit.move_to_previous_line(KeyCode::Null, &mut state);

        assert_eq!(edit.cursor_index, 5);
    }

    #[test]
    fn next_line_longer_than_previous_with_additional_line() {
        let mut edit = TextEditPanel::new();
        edit.set_text("12345\n1234567890\n12345".to_string());
        edit.cursor_index = 19;
        let mut state = AppState::new();

        edit.move_to_previous_line(KeyCode::Null, &mut state);

        assert_eq!(edit.cursor_index, 8);
    }
}
