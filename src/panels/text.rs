use std::{fs, iter};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use crossterm::event::{KeyCode, KeyEvent};
use tui::layout::{Direction, Rect};
use tui::text::{Span, Spans, Text};
use crate::{AppState, catch_all, CommandDetails, Commands, ctrl_key, CURSOR_MAX, EditorFrame};
use crate::app::{Message, StateChangeRequest};
use crate::autocomplete::FileAutoCompleter;
use crate::commands::{alt_key, shift_alt_key, shift_catch_all};
use crate::panels::{EDIT_PANEL_TYPE_ID, INPUT_PANEL_TYPE_ID, InputPanel, MESSAGE_PANEL_TYPE_ID, MessagesPanel, NULL_PANEL_TYPE_ID, PanelFactory, PanelTypeID};
use crate::panels::edit::TextEditPanel;

#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
pub enum PanelState {
    Normal,
    WaitingToOpen,
    WaitingToSave,
}

pub struct TextPanel {
    current_line: usize,
    cursor_index_in_line: usize,
    title: String,
    file_path: Option<PathBuf>,
    scroll_y: u16,
    lines: Vec<String>,
    gutter_size: u16,
    visible: bool,
    panel_type: PanelTypeID,
    state: PanelState,
    continuation_marker: String,
    selection: usize,
    command_index: usize,
    pub(crate) length_handler: fn(&TextPanel, u16, u16, Direction, &AppState) -> u16,
    pub(crate) receive_input_handler: fn(&mut TextPanel, String) -> Vec<StateChangeRequest>,
    pub(crate) render_handler: fn(&TextPanel, &AppState, &mut EditorFrame, Rect),
}

impl Default for TextPanel {
    fn default() -> Self {
        Self {
            current_line: 0,
            cursor_index_in_line: 0,
            title: String::new(),
            file_path: None,
            scroll_y: 0,
            lines: vec![],
            gutter_size: 5,
            visible: true,
            panel_type: NULL_PANEL_TYPE_ID,
            state: PanelState::Normal,
            continuation_marker: "... ".to_string(),
            selection: 0,
            command_index: 0,
            length_handler: TextPanel::empty_length_handler,
            receive_input_handler: TextPanel::empty_input_handler,
            render_handler: TextPanel::empty_render_handler,
        }
    }
}

impl TextPanel {

    fn empty_length_handler(_: &TextPanel, _: u16, _: u16, _: Direction, _: &AppState) -> u16 {
        0
    }

    fn empty_input_handler(_: &mut TextPanel, _: String) -> Vec<StateChangeRequest> {
        vec![]
    }

    fn empty_render_handler(_: &TextPanel, _: &AppState, _: &mut EditorFrame, _: Rect) {
        // RenderDetails::new(vec![], (0, 0))
    }

    pub fn edit_panel() -> Self {
        let mut defaults = TextPanel::default();
        defaults.panel_type = EDIT_PANEL_TYPE_ID;

        defaults.render_handler = TextEditPanel::render_handler;
        defaults.receive_input_handler = TextEditPanel::input_handler;

        defaults
    }

    pub fn input_panel() -> Self {
        let mut defaults = TextPanel::default();
        defaults.panel_type = INPUT_PANEL_TYPE_ID;

        defaults.render_handler = TextEditPanel::render_handler;
        defaults.length_handler = InputPanel::length_handler;

        defaults
    }

    pub fn messages_panel() -> Self {
        let mut defaults = TextPanel::default();
        defaults.panel_type = MESSAGE_PANEL_TYPE_ID;

        defaults.render_handler = MessagesPanel::render_handler;

        defaults
    }

    fn init(&mut self, _state: &mut AppState) {

    }

    // temp
    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    pub fn set_text<T: ToString>(&mut self, text: T) {
        self.lines = text.to_string().split('\n').map(|s| s.to_string()).collect();
    }

    pub fn append_text<T: ToString>(&mut self, text: T) {
        let new_lines = text.to_string();
        let mut spliterator = new_lines.split('\n');

        match spliterator.next() {
            None => return,
            Some(line) => {
                // append first line of new text to last line of existing text
                // first line is empty if starts with newline
                match self.lines.get_mut(self.current_line) {
                    None => {
                        self.lines.push(line.to_string());
                    }
                    Some(existing) => existing.extend(line.chars()),
                }
            }
        }

        for line in spliterator {
            // append remaining lines as new
            self.lines.push(line.to_string());
        }
    }

    pub fn lines(&self) -> &Vec<String> {
        &self.lines
    }

    pub fn selection(&self) -> usize {
        self.selection
    }

    pub fn set_selection(&mut self, selection: usize) {
        self.selection = selection;
    }

    pub fn title(&self) -> &String {
        &self.title
    }

    pub fn set_title(&mut self, title: String) {
        self.title = title;
    }

    pub fn current_line(&self) -> usize {
        self.current_line
    }

    pub fn set_current_line(&mut self, current_line: usize) {
        self.current_line = current_line;
    }

    pub fn cursor_index_in_line(&self) -> usize {
        self.cursor_index_in_line
    }

    pub fn set_cursor_index(&mut self, index: usize) {
        self.cursor_index_in_line = index;
    }

    pub fn scroll_y(&self) -> u16 {
        self.scroll_y
    }

    pub fn set_scroll_y(&mut self, y: u16) {
        self.scroll_y = y;
    }

    pub fn state(&self) -> PanelState {
        self.state
    }

    pub fn file_path(&self) -> Option<&PathBuf> {
        self.file_path.as_ref()
    }

    pub fn set_file_path(&mut self, path: PathBuf) {
        self.file_path = Some(path);
    }

    pub fn gutter_size(&self) -> u16 {
        self.gutter_size
    }

    pub fn continuation_marker(&self) -> &String {
        &self.continuation_marker
    }

    pub fn panel_type(&self) -> PanelTypeID {
        self.panel_type
    }

    pub fn show(&mut self) {
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn visible(&self) -> bool {
        self.visible
    }

    pub fn make_widget(
        &self,
        state: &AppState,
        frame: &mut EditorFrame,
        rect: Rect
    ) {
        (self.render_handler)(self, state, frame, rect)
    }

    pub fn get_length(
        &self,
        fixed_length: u16,
        flex_length: u16,
        direction: Direction,
        state: &AppState,
    ) -> u16 {
        (self.length_handler)(self, fixed_length, flex_length, direction, state)
    }

    pub fn receive_input(&mut self, input: String) -> Vec<StateChangeRequest> {
        (self.receive_input_handler)(self, input)
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

    pub(crate) fn handle_key_stroke(
        &mut self,
        code: KeyCode,
        state: &mut AppState,
    ) -> (bool, Vec<StateChangeRequest>) {
        self.handle_key_stroke_internal(code, state, TextPanel::enter_newline)
    }

    pub(crate) fn handle_key_stroke_internal<Enter>(
        &mut self,
        code: KeyCode,
        state: &mut AppState,
        enter_func: Enter,
    ) -> (bool, Vec<StateChangeRequest>)
    where Enter: FnOnce(&mut TextPanel, &mut Vec<StateChangeRequest>)
    {
        let mut changes = vec![];
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
                enter_func(self, &mut changes)
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

        (true, changes)
    }

    pub fn enter_newline(&mut self, _: &mut Vec<StateChangeRequest>) {
        self.lines.push(String::new());
        self.current_line += 1;
        self.cursor_index_in_line = 0;
    }

    pub(crate) fn open_file(
        &mut self,
        _code: KeyCode,
        _state: &mut AppState,
    ) -> (bool, Vec<StateChangeRequest>) {
        self.state = PanelState::WaitingToOpen;
        (
            true,
            vec![StateChangeRequest::input_request_with_completer(
                "File Name".to_string(),
                Box::new(FileAutoCompleter::new()),
            )],
        )
    }

    pub fn set_cursor_to_end(&mut self) {
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

    pub(crate) fn move_to_next_character(
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

    pub(crate) fn move_to_previous_character(
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

    pub(crate) fn move_to_next_line(
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

    pub(crate) fn move_to_previous_line(
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

    pub(crate) fn scroll_down_one(
        &mut self,
        _code: KeyCode,
        _state: &mut AppState,
    ) -> (bool, Vec<StateChangeRequest>) {
        self.scroll_down(1);
        (true, vec![])
    }

    pub(crate) fn scroll_up_one(
        &mut self,
        _code: KeyCode,
        _state: &mut AppState,
    ) -> (bool, Vec<StateChangeRequest>) {
        self.scroll_up(1);
        (true, vec![])
    }

    pub(crate) fn scroll_down_ten(
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

    pub(crate) fn scroll_up_ten(
        &mut self,
        _code: KeyCode,
        _state: &mut AppState,
    ) -> (bool, Vec<StateChangeRequest>) {
        self.scroll_up(10);
        (true, vec![])
    }

    pub fn make_text_content(&self, text_content_box: Rect) -> (Vec<Spans>, (u16, u16), Vec<Spans>) {
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

    pub(crate) fn save_buffer(
        &mut self,
        _code: KeyCode,
        _state: &mut AppState,
    ) -> (bool, Vec<StateChangeRequest>) {
        (true, self.save())
    }

    pub fn save(&mut self) -> Vec<StateChangeRequest> {
        let mut changes = vec![];

        match &self.file_path {
            None => {
                self.state = PanelState::WaitingToSave;
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

                match File::options()
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

pub type PanelCommand =
fn(&mut TextPanel, KeyCode, &mut AppState) -> (bool, Vec<StateChangeRequest>);

// unwarps allowed here for now because there shouldn't be any misconfigurations in default settings
pub fn make_edit_commands() -> Commands<PanelCommand> {
    let mut commands = Commands::<PanelCommand>::new();

    commands.insert(|b| {
        b.node(catch_all())
            .action(CommandDetails::empty(), TextPanel::handle_key_stroke)
    }).unwrap();

    commands.insert(|b| {
        b.node(shift_catch_all())
            .action(CommandDetails::empty(), TextPanel::handle_key_stroke)
    }).unwrap();

    commands.insert(|b| {
        b.node(ctrl_key('o'))
            .action(CommandDetails::open_file(), TextPanel::open_file)
    }).unwrap();

    commands.insert(|b| {
        b.node(ctrl_key('s'))
            .action(CommandDetails::empty(), TextPanel::save_buffer)
    }).unwrap();

    commands.insert(|b| {
        b.node(alt_key('i'))
            .action(CommandDetails::empty(), TextPanel::scroll_up_one)
    }).unwrap();

    commands.insert(|b| {
        b.node(alt_key('k'))
            .action(CommandDetails::empty(), TextPanel::scroll_down_one)
    }).unwrap();

    commands.insert(|b| {
        b.node(shift_alt_key('I'))
            .action(CommandDetails::empty(), TextPanel::scroll_up_ten)
    }).unwrap();

    commands.insert(|b| {
        b.node(shift_alt_key('K'))
            .action(CommandDetails::empty(), TextPanel::scroll_down_ten)
    }).unwrap();

    commands.insert(|b| {
        b.node(alt_key('w')).action(
            CommandDetails::empty(),
            TextPanel::move_to_previous_line,
        )
    }).unwrap();

    commands.insert(|b| {
        b.node(alt_key('a')).action(
            CommandDetails::empty(),
            TextPanel::move_to_previous_character,
        )
    }).unwrap();

    commands.insert(|b| {
        b.node(alt_key('s'))
            .action(CommandDetails::empty(), TextPanel::move_to_next_line)
    }).unwrap();

    commands.insert(|b| {
        b.node(alt_key('d')).action(
            CommandDetails::empty(),
            TextPanel::move_to_next_character,
        )
    }).unwrap();

    commands
}