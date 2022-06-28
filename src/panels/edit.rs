use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::{env, iter};

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

enum EditState {
    Normal,
    WaitingToOpen,
    WaitingToSave,
}

pub struct TextEditPanel {
    current_line: usize,
    cursor_index_in_line: usize,
    title: String,
    commands: Commands<EditCommand>,
    file_path: Option<PathBuf>,
    gutter_size: u16,
    continuation_marker: String,
    scroll_y: u16,
    lines: Vec<String>,
    state: EditState,
}

#[allow(dead_code)]
impl TextEditPanel {
    pub fn new() -> Self {
        TextEditPanel {
            scroll_y: 0,
            current_line: 0,
            cursor_index_in_line: 0,
            gutter_size: 5,
            title: "Buffer".to_string(),
            commands: Commands::<EditCommand>::new(),
            file_path: None,
            continuation_marker: "... ".to_string(),
            lines: vec![],
            state: EditState::Normal,
        }
    }

    pub fn set_text<T: ToString>(&mut self, text: T) {
        for line in text.to_string().split('\n') {
            self.lines.push(line.to_string());
        }
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
                if self.cursor_index_in_line == 0 {
                    self.remove_line();
                } else {
                    self.remove_character(1, 1, state);
                }
            }
            KeyCode::Delete => match self.lines.get(self.current_line) {
                None => (),
                Some(line) => {
                    if self.cursor_index_in_line == line.len() {
                        self.current_line += 1;
                        self.remove_line();
                    } else {
                        self.remove_character(0, 0, state);
                    }
                }
            },
            KeyCode::Enter => {
                self.lines.push(String::new());
                self.current_line += 1;
                self.cursor_index_in_line = 0;
            }
            KeyCode::Char(c) => {
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
        _state: &mut AppState,
    ) -> (bool, Vec<StateChangeRequest>) {
        self.state = EditState::WaitingToOpen;
        (
            true,
            vec![StateChangeRequest::input_request_with_completer(
                "File Name".to_string(),
                Box::new(FileAutoCompleter::new()),
            )],
        )
    }

    fn set_cursor_to_end(&mut self) {
        if self.lines.len() > 0 {
            self.current_line = self.lines.len() - 1;
            self.cursor_index_in_line = match self.lines.get(self.current_line) {
                None => 0,
                Some(line) => line.len(),
            };
        } else {
            self.current_line = 0;
            self.cursor_index_in_line = 0;
        }
    }

    fn move_to_next_character(
        &mut self,
        _code: KeyCode,
        _state: &mut AppState,
    ) -> (bool, Vec<StateChangeRequest>) {
        match self.lines.get(self.current_line) {
            None => self.cursor_index_in_line = 0,
            Some(line) => {
                if self.cursor_index_in_line + 1 > line.len()
                    && self.current_line + 1 < self.lines.len()
                {
                    self.cursor_index_in_line = 0;
                    self.current_line += 1;
                } else {
                    self.cursor_index_in_line += 1;
                }
            }
        }

        (true, vec![])
    }

    fn move_to_previous_character(
        &mut self,
        _code: KeyCode,
        _state: &mut AppState,
    ) -> (bool, Vec<StateChangeRequest>) {
        if self.cursor_index_in_line > 0 {
            self.cursor_index_in_line -= 1;
        } else if self.current_line > 0 {
            self.current_line -= 1;
            self.cursor_index_in_line = match self.lines.get(self.current_line) {
                None => 0,
                Some(l) => l.len(),
            }
        }

        (true, vec![])
    }

    fn move_to_next_line(
        &mut self,
        _code: KeyCode,
        _state: &mut AppState,
    ) -> (bool, Vec<StateChangeRequest>) {
        if self.current_line + 1 < self.lines.len() {
            self.current_line += 1;

            match self.lines.get(self.current_line) {
                None => self.cursor_index_in_line = 0,
                Some(line) => {
                    if self.cursor_index_in_line > line.len() {
                        self.cursor_index_in_line = line.len();
                    }
                }
            }
        }

        (true, vec![])
    }

    fn move_to_previous_line(
        &mut self,
        _code: KeyCode,
        _state: &mut AppState,
    ) -> (bool, Vec<StateChangeRequest>) {
        if self.current_line > 0 {
            self.current_line -= 1;

            match self.lines.get(self.current_line) {
                None => self.cursor_index_in_line = 0,
                Some(line) => {
                    if self.cursor_index_in_line > line.len() {
                        self.cursor_index_in_line = line.len();
                    }
                }
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
        let limit = self.lines.len() as u16;
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
        let mut real_line_count = self.scroll_y;

        for i in 0..(text_content_box.height) {
            let true_index = (i + self.scroll_y) as usize;
            real_line_count += 1;

            match self.lines.get(true_index) {
                None => (), // empty
                Some(line) => {
                    if line.len() < max_text_length {
                        lines.push(Spans::from(line.as_str()));
                        gutter.push(Spans::from(Span::from(real_line_count.to_string())));

                        if true_index == self.current_line {
                            cursor_y = text_content_box.y + lines.len() as u16 - 1;
                            cursor_x = text_content_box.x + self.cursor_index_in_line as u16;
                        }
                    } else {
                        let starting_lines = lines.len();
                        let (mut current, mut next) = line.split_at(max_text_length);
                        let continuation_length = max_text_length - self.continuation_marker.len();

                        lines.push(Spans::from(Span::from(current)));
                        gutter.push(Spans::from(Span::from(real_line_count.to_string())));

                        while next.len() >= continuation_length {
                            (current, next) = next.split_at(continuation_length);

                            lines.push(Spans::from(vec![
                                Span::from(self.continuation_marker.as_str()),
                                Span::from(current),
                            ]));
                            gutter.push(Spans::from(Span::from(".")));
                        }

                        lines.push(Spans::from(vec![
                            Span::from(self.continuation_marker.as_str()),
                            Span::from(next),
                        ]));
                        gutter.push(Spans::from(Span::from(".")));

                        if true_index == self.current_line {
                            let continuation_count = lines.len() - starting_lines - 1;
                            let mut cursor_position = self.cursor_index_in_line;
                            for amount in iter::once(max_text_length)
                                .chain(iter::repeat(continuation_length).take(continuation_count))
                            {
                                if cursor_position <= amount {
                                    break;
                                }

                                cursor_position -= amount;
                            }

                            cursor_y = text_content_box.y + lines.len() as u16 - 1;
                            cursor_x = text_content_box.x
                                + self.continuation_marker.len() as u16
                                + cursor_position as u16;
                        }
                    }
                }
            }
        }

        (lines, (cursor_x, cursor_y), gutter)
    }

    fn save_buffer(
        &mut self,
        _code: KeyCode,
        _state: &mut AppState,
    ) -> (bool, Vec<StateChangeRequest>) {
        (true, self.save())
    }

    fn save(&mut self) -> Vec<StateChangeRequest> {
        let mut changes = vec![];

        match &self.file_path {
            None => {
                self.state = EditState::WaitingToSave;
                return vec![StateChangeRequest::input_request_with_completer(
                    "File Name".to_string(),
                    Box::new(FileAutoCompleter::new()),
                )];
            }
            Some(file_path) => {
                changes.push(StateChangeRequest::info(format!(
                    "Saving file to {:?}",
                    file_path
                )));

                match fs::File::options()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(file_path)
                {
                    Err(err) => {
                        changes.push(StateChangeRequest::error(format!(
                            "Could not open file to save. {}",
                            err.to_string()
                        )));
                    }
                    Ok(mut file) => {
                        self.lines.iter().for_each(|line| {
                            match file.write(line.as_bytes()) {
                                Err(err) => changes.push(StateChangeRequest::error(format!(
                                    "Could not write to file. {}",
                                    err.to_string()
                                ))),
                                Ok(_) => (),
                            }
                            match file.write("\n".as_bytes()) {
                                Err(err) => changes.push(StateChangeRequest::error(format!(
                                    "Could not write to file. {}",
                                    err.to_string()
                                ))),
                                Ok(_) => (),
                            }
                        });

                        changes.push(StateChangeRequest::info("Save complete."));
                    }
                }
            }
        }

        changes
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
        if !self.lines.is_empty() {
            let line_count = self.lines.len();
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

        match self.state {
            EditState::WaitingToOpen => {
                let current_dir = match env::current_dir() {
                    Err(e) => {
                        changes.push(StateChangeRequest::error(e));
                        return changes;
                    }
                    Ok(p) => p,
                };

                let mut file_path = (&current_dir).clone();
                file_path.push(input);

                match fs::File::open(&file_path) {
                    Err(e) => changes.push(StateChangeRequest::error(e)),
                    Ok(mut file) => {
                        let mut s = String::new();
                        match file.read_to_string(&mut s) {
                            Err(e) => changes.push(StateChangeRequest::error(e)),
                            Ok(_) => {
                                self.set_text(s);

                                self.title = if file_path.starts_with(&current_dir) {
                                    match file_path.strip_prefix(&current_dir) {
                                        Err(e) => {
                                            changes.push(StateChangeRequest::error(e));
                                            file_path.to_string_lossy().to_string()
                                        }
                                        Ok(p) => p.as_os_str().to_string_lossy().to_string(),
                                    }
                                } else {
                                    file_path.to_string_lossy().to_string()
                                }
                            }
                        }
                        self.file_path = Some(file_path.clone());
                    }
                };

                self.scroll_y = 0;
            }
            EditState::WaitingToSave => {
                let current_dir = match env::current_dir() {
                    Err(e) => {
                        changes.push(StateChangeRequest::error(e));
                        return changes;
                    }
                    Ok(p) => p,
                };

                let mut file_path = (&current_dir).clone();
                file_path.push(input);
                self.file_path = Some(file_path.clone());

                changes.extend(self.save());
            }
            EditState::Normal => (),
        }

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
        b.node(ctrl_key('s'))
            .action(CommandDetails::empty(), TextEditPanel::save_buffer)
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

    commands.insert(|b| {
        b.node(alt_key('w')).action(
            CommandDetails::empty(),
            TextEditPanel::move_to_previous_line,
        )
    })?;

    commands.insert(|b| {
        b.node(alt_key('a')).action(
            CommandDetails::empty(),
            TextEditPanel::move_to_previous_character,
        )
    })?;

    commands.insert(|b| {
        b.node(alt_key('s'))
            .action(CommandDetails::empty(), TextEditPanel::move_to_next_line)
    })?;

    commands.insert(|b| {
        b.node(alt_key('d')).action(
            CommandDetails::empty(),
            TextEditPanel::move_to_next_character,
        )
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
    fn set_text() {
        let mut edit = TextEditPanel::new();
        edit.set_text("\n123456789\n123456\n");

        assert_eq!(
            edit.lines,
            vec![
                "".to_string(),
                "123456789".to_string(),
                "123456".to_string(),
                "".to_string()
            ]
        )
    }

    #[test]
    fn cursor_is_one_past_end() {
        let mut edit = TextEditPanel::new();
        edit.set_text("123456789\n123456");
        edit.set_cursor_to_end();

        let (_, cursor, _) = edit.make_text_content(Rect::new(10, 10, 20, 20));

        assert_eq!(cursor, (16, 11));
    }

    #[test]
    fn cursor_on_continuation_line() {
        let mut edit = TextEditPanel::new();
        edit.set_text("123456789012345678901234567890");
        edit.current_line = 0;
        edit.cursor_index_in_line = 25;

        let (_, cursor, _) = edit.make_text_content(Rect::new(10, 10, 20, 20));

        assert_eq!(
            cursor,
            (
                edit.cursor_index_in_line as u16 - 10 + edit.continuation_marker.len() as u16,
                11
            )
        );
    }

    #[test]
    fn cursor_end_of_continuation_line() {
        let mut edit = TextEditPanel::new();
        edit.set_text("123456789012345678901234567890");
        edit.set_cursor_to_end();

        let (_, cursor, _) = edit.make_text_content(Rect::new(10, 10, 20, 20));

        assert_eq!(cursor, (20 + edit.continuation_marker.len() as u16, 11));
    }

    #[test]
    fn cursor_end_of_multiple_continuation_line() {
        let mut edit = TextEditPanel::new();
        //           |                   |               |
        edit.set_text("12345678901234567890123456789012345678901234567890");
        edit.set_cursor_to_end();

        let (_, cursor, _) = edit.make_text_content(Rect::new(10, 10, 20, 20));

        assert_eq!(cursor, (24 + edit.continuation_marker.len() as u16, 12));
    }

    #[test]
    fn line_after_line_with_continuations() {
        let mut edit = TextEditPanel::new();
        //           |                   |               |
        edit.set_text("12345678901234567890123456789012345678901234567890\n1234567890");
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
    fn cursor_is_next_line_when_after_newline() {
        let mut edit = TextEditPanel::new();
        edit.set_text("123456789\n123456\n");
        edit.set_cursor_to_end();

        let (_, cursor, _) = edit.make_text_content(Rect::new(10, 10, 20, 20));

        assert_eq!(cursor, (10, 12));
    }

    #[test]
    fn newline_after_line_with_continuations() {
        let mut edit = TextEditPanel::new();
        //           |                   |               |
        edit.set_text("12345678901234567890123456789012345678901234567890\n");
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
        edit.set_text(
            (100..200)
                .map(|i| i.to_string())
                .collect::<Vec<String>>()
                .join("\n"),
        );
        edit.current_line = 12;
        edit.cursor_index_in_line = 1;
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
        edit.set_text(
            (100..200)
                .map(|i| i.to_string())
                .collect::<Vec<String>>()
                .join("\n"),
        );
        edit.scroll_y = 95;

        let mut state = AppState::new();

        edit.scroll_down_one(KeyCode::Null, &mut state);

        assert_eq!(edit.scroll_y, 96);
    }

    #[test]
    fn scroll_down_past_text() {
        let mut edit = TextEditPanel::new();
        edit.set_text(
            (100..200)
                .map(|i| i.to_string())
                .collect::<Vec<String>>()
                .join("\n"),
        );
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
    fn next_character() {
        let mut edit = TextEditPanel::new();
        edit.set_text(
            (100..200)
                .map(|i| i.to_string())
                .collect::<Vec<String>>()
                .join("\n"),
        );
        edit.cursor_index_in_line = 2;
        edit.current_line = 2;
        let mut state = AppState::new();

        edit.move_to_next_character(KeyCode::Null, &mut state);

        assert_eq!(edit.cursor_index_in_line, 3);
    }

    #[test]
    fn next_character_to_next_line() {
        let mut edit = TextEditPanel::new();
        edit.set_text(
            (100..200)
                .map(|i| i.to_string())
                .collect::<Vec<String>>()
                .join("\n"),
        );
        edit.cursor_index_in_line = 2;
        edit.current_line = 2;
        let mut state = AppState::new();

        edit.move_to_next_character(KeyCode::Null, &mut state);
        assert_eq!(edit.cursor_index_in_line, 3);

        edit.move_to_next_character(KeyCode::Null, &mut state);
        assert_eq!(edit.cursor_index_in_line, 0);
        assert_eq!(edit.current_line, 3);
    }

    #[test]
    fn previous_character() {
        let mut edit = TextEditPanel::new();
        edit.set_text(
            (100..200)
                .map(|i| i.to_string())
                .collect::<Vec<String>>()
                .join("\n"),
        );
        edit.cursor_index_in_line = 2;
        edit.current_line = 2;
        let mut state = AppState::new();

        edit.move_to_previous_character(KeyCode::Null, &mut state);

        assert_eq!(edit.cursor_index_in_line, 1);
    }

    #[test]
    fn previous_character_to_previous_line() {
        let mut edit = TextEditPanel::new();
        edit.set_text(
            (100..200)
                .map(|i| i.to_string())
                .collect::<Vec<String>>()
                .join("\n"),
        );
        edit.cursor_index_in_line = 2;
        edit.current_line = 2;
        let mut state = AppState::new();

        edit.move_to_previous_character(KeyCode::Null, &mut state);
        assert_eq!(edit.cursor_index_in_line, 1);

        edit.move_to_previous_character(KeyCode::Null, &mut state);
        assert_eq!(edit.cursor_index_in_line, 0);

        edit.move_to_previous_character(KeyCode::Null, &mut state);
        assert_eq!(edit.cursor_index_in_line, 3);
        assert_eq!(edit.current_line, 1);
    }

    #[test]
    fn previous_character_at_zero() {
        let mut edit = TextEditPanel::new();
        edit.set_text(
            (100..200)
                .map(|i| i.to_string())
                .collect::<Vec<String>>()
                .join("\n"),
        );
        let mut state = AppState::new();

        edit.move_to_previous_character(KeyCode::Null, &mut state);

        assert_eq!(edit.cursor_index_in_line, 0);
        assert_eq!(edit.current_line, 0);
    }

    #[test]
    fn next_line() {
        let mut edit = TextEditPanel::new();
        edit.set_text("12345\n12345".to_string());
        edit.cursor_index_in_line = 4;
        edit.current_line = 0;
        let mut state = AppState::new();

        edit.move_to_next_line(KeyCode::Null, &mut state);

        assert_eq!(edit.cursor_index_in_line, 4);
        assert_eq!(edit.current_line, 1);
    }

    #[test]
    fn next_line_no_line() {
        let mut edit = TextEditPanel::new();
        edit.set_text("12345\n12345".to_string());
        edit.cursor_index_in_line = 4;
        edit.current_line = 1;
        let mut state = AppState::new();

        edit.move_to_next_line(KeyCode::Null, &mut state);

        assert_eq!(edit.cursor_index_in_line, 4);
        assert_eq!(edit.current_line, 1);
    }

    #[test]
    fn next_line_longer_than_next() {
        let mut edit = TextEditPanel::new();
        edit.set_text("1234567890\n12345\n1234567890".to_string());
        edit.cursor_index_in_line = 9;
        edit.current_line = 0;
        let mut state = AppState::new();

        edit.move_to_next_line(KeyCode::Null, &mut state);

        assert_eq!(edit.cursor_index_in_line, 5);
        assert_eq!(edit.current_line, 1);
    }

    #[test]
    fn next_line_that_is_last_line() {
        let mut edit = TextEditPanel::new();
        edit.set_text("1234567890\n12345\n1234567890".to_string());
        edit.cursor_index_in_line = 4;
        edit.current_line = 1;
        let mut state = AppState::new();

        edit.move_to_next_line(KeyCode::Null, &mut state);

        assert_eq!(edit.cursor_index_in_line, 4);
        assert_eq!(edit.current_line, 2);
    }

    #[test]
    fn previous_line() {
        let mut edit = TextEditPanel::new();
        edit.set_text("1234567890\n1234567890".to_string());
        edit.cursor_index_in_line = 4;
        edit.current_line = 1;
        let mut state = AppState::new();

        edit.move_to_previous_line(KeyCode::Null, &mut state);

        assert_eq!(edit.cursor_index_in_line, 4);
        assert_eq!(edit.current_line, 0);
    }

    #[test]
    fn next_line_longer_than_previous() {
        let mut edit = TextEditPanel::new();
        edit.set_text("12345\n1234567890".to_string());
        edit.cursor_index_in_line = 9;
        edit.current_line = 1;
        let mut state = AppState::new();

        edit.move_to_previous_line(KeyCode::Null, &mut state);

        assert_eq!(edit.cursor_index_in_line, 5);
        assert_eq!(edit.current_line, 0);
    }
}
