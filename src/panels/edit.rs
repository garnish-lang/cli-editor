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
use crate::{catch_all, ctrl_key, AppState, CommandDetails, CommandKeyId, Commands, EditorFrame, CURSOR_MAX, TextPanel};
use crate::panels::text::PanelState;

pub struct TextEditPanel {}

#[allow(dead_code)]
impl TextEditPanel {
    pub fn input_handler(panel: &mut TextPanel, input: String) -> Vec<StateChangeRequest> {
        let mut changes = vec![];

        match panel.state() {
            PanelState::WaitingToOpen => {
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
                                panel.set_text(s);

                                panel.set_title(if file_path.starts_with(&current_dir) {
                                    match file_path.strip_prefix(&current_dir) {
                                        Err(e) => {
                                            changes.push(StateChangeRequest::error(e));
                                            file_path.to_string_lossy().to_string()
                                        }
                                        Ok(p) => p.as_os_str().to_string_lossy().to_string(),
                                    }
                                } else {
                                    file_path.to_string_lossy().to_string()
                                });
                            }
                        }
                        panel.set_file_path(file_path.clone());
                    }
                };

                panel.set_scroll_y(0);
            }
            PanelState::WaitingToSave => {
                let current_dir = match env::current_dir() {
                    Err(e) => {
                        changes.push(StateChangeRequest::error(e));
                        return changes;
                    }
                    Ok(p) => p,
                };

                let mut file_path = (&current_dir).clone();
                file_path.push(input);
                panel.set_file_path(file_path.clone());

                changes.extend(panel.save());
            }
            PanelState::Normal => (),
        }

        changes
    }

    pub fn render_handler(panel: &TextPanel, _state: &AppState, frame: &mut EditorFrame, rect: Rect) {
        if !panel.lines().is_empty() {
            let line_count = panel.lines().len();
            let line_count_size = line_count.to_string().len().min(u16::MAX as usize) as u16;

            let layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(vec![
                    Constraint::Length(line_count_size),
                    Constraint::Length(panel.gutter_size()),
                    Constraint::Length(rect.width - line_count_size - panel.gutter_size()),
                ])
                .split(rect);

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
        }
    }
}


#[cfg(test)]
mod tests {
    use crossterm::event::KeyCode;
    use tui::layout::Rect;
    use tui::text::{Span, Spans};

    use crate::{AppState, TextPanel};
    use crate::panels::edit::TextEditPanel;

    #[test]
    fn set_text() {
        let mut edit = TextPanel::default();
        edit.set_text("\n123456789\n123456\n");

        assert_eq!(
            edit.lines(),
            &vec![
                "".to_string(),
                "123456789".to_string(),
                "123456".to_string(),
                "".to_string()
            ]
        )
    }

    #[test]
    fn cursor_is_one_past_end() {
        let mut edit = TextPanel::default();
        edit.set_text("123456789\n123456");
        edit.set_cursor_to_end();

        let (_, cursor, _) = edit.make_text_content(Rect::new(10, 10, 20, 20));

        assert_eq!(cursor, (16, 11));
    }

    #[test]
    fn cursor_on_continuation_line() {
        let mut edit = TextPanel::default();
        edit.set_text("123456789012345678901234567890");
        edit.set_current_line(0);
        edit.set_cursor_index(25);

        let (_, cursor, _) = edit.make_text_content(Rect::new(10, 10, 20, 20));

        assert_eq!(
            cursor,
            (
                edit.cursor_index_in_line() as u16 - 10 + edit.continuation_marker().len() as u16,
                11
            )
        );
    }

    #[test]
    fn cursor_end_of_continuation_line() {
        let mut edit = TextPanel::default();
        edit.set_text("123456789012345678901234567890");
        edit.set_cursor_to_end();

        let (_, cursor, _) = edit.make_text_content(Rect::new(10, 10, 20, 20));

        assert_eq!(cursor, (20 + edit.continuation_marker().len() as u16, 11));
    }

    #[test]
    fn cursor_end_of_multiple_continuation_line() {
        let mut edit = TextPanel::default();
        //           |                   |               |
        edit.set_text("12345678901234567890123456789012345678901234567890");
        edit.set_cursor_to_end();

        let (_, cursor, _) = edit.make_text_content(Rect::new(10, 10, 20, 20));

        assert_eq!(cursor, (24 + edit.continuation_marker().len() as u16, 12));
    }

    #[test]
    fn line_after_line_with_continuations() {
        let mut edit = TextPanel::default();
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
        let mut edit = TextPanel::default();
        edit.set_text("123456789\n123456\n");
        edit.set_cursor_to_end();

        let (_, cursor, _) = edit.make_text_content(Rect::new(10, 10, 20, 20));

        assert_eq!(cursor, (10, 12));
    }

    #[test]
    fn newline_after_line_with_continuations() {
        let mut edit = TextPanel::default();
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
        let mut edit = TextPanel::default();
        edit.set_text(
            (100..200)
                .map(|i| i.to_string())
                .collect::<Vec<String>>()
                .join("\n"),
        );
        edit.set_current_line(12);
        edit.set_cursor_index(1);
        edit.set_scroll_y(10);

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
        let mut edit = TextPanel::default();
        let mut state = AppState::new();

        edit.handle_key_stroke(KeyCode::Char('a'), &mut state);

        assert_eq!(edit.lines(), &vec!["a".to_string()]);
        assert_eq!(edit.cursor_index_in_line(), 1);
    }

    #[test]
    fn handle_character_key_middle_of_line() {
        let mut edit = TextPanel::default();
        edit.set_text("ac");
        edit.set_cursor_index(1);

        let mut state = AppState::new();

        edit.handle_key_stroke(KeyCode::Char('b'), &mut state);

        assert_eq!(edit.lines(), &vec!["abc".to_string()]);
        assert_eq!(edit.cursor_index_in_line(), 2);
    }

    #[test]
    fn handle_character_key_with_existing_text() {
        let mut edit = TextPanel::default();
        edit.set_text("a");
        edit.set_cursor_index(1);

        let mut state = AppState::new();

        edit.handle_key_stroke(KeyCode::Char('b'), &mut state);

        assert_eq!(edit.lines(), &vec!["ab".to_string()]);
        assert_eq!(edit.cursor_index_in_line(), 2);
    }

    #[test]
    fn handle_enter_key() {
        let mut edit = TextPanel::default();
        edit.set_text("a");
        edit.set_cursor_index(1);

        let mut state = AppState::new();

        edit.handle_key_stroke(KeyCode::Enter, &mut state);

        assert_eq!(edit.lines(), &vec!["a".to_string(), String::new()]);
        assert_eq!(edit.current_line(), 1);
        assert_eq!(edit.cursor_index_in_line(), 0);
    }

    #[test]
    fn handle_backspace_key() {
        let mut edit = TextPanel::default();
        edit.set_text("a");
        edit.set_cursor_index(1);

        let mut state = AppState::new();

        edit.handle_key_stroke(KeyCode::Backspace, &mut state);

        assert_eq!(edit.lines(), &vec!["".to_string()]);
        assert_eq!(edit.current_line(), 0);
        assert_eq!(edit.cursor_index_in_line(), 0);
    }

    #[test]
    fn handle_backspace_key_middle_of_line() {
        let mut edit = TextPanel::default();
        edit.set_text("abc");
        edit.set_cursor_index(2);

        let mut state = AppState::new();

        edit.handle_key_stroke(KeyCode::Backspace, &mut state);

        assert_eq!(edit.lines(), &vec!["ac".to_string()]);
        assert_eq!(edit.current_line(), 0);
        assert_eq!(edit.cursor_index_in_line(), 1);
    }

    #[test]
    fn handle_backspace_key_start_of_non_empty_line() {
        let mut edit = TextPanel::default();
        edit.set_text("abc\ndef");
        edit.set_current_line(1);
        edit.set_cursor_index(0);

        let mut state = AppState::new();

        edit.handle_key_stroke(KeyCode::Backspace, &mut state);

        assert_eq!(edit.lines(), &vec!["abcdef".to_string()]);
        assert_eq!(edit.current_line(), 0);
        assert_eq!(edit.cursor_index_in_line(), 3);
    }

    #[test]
    fn handle_backspace_key_on_newline() {
        let mut edit = TextPanel::default();
        edit.set_text("a\na");
        edit.set_current_line(1);
        edit.set_cursor_index(1);

        let mut state = AppState::new();

        edit.handle_key_stroke(KeyCode::Backspace, &mut state);
        edit.handle_key_stroke(KeyCode::Backspace, &mut state);

        assert_eq!(edit.lines(), &vec!["a".to_string()]);
        assert_eq!(edit.current_line(), 0);
        assert_eq!(edit.cursor_index_in_line(), 1);
    }

    #[test]
    fn handle_delete_key() {
        let mut edit = TextPanel::default();
        edit.set_text("a");
        edit.set_cursor_index(0);

        let mut state = AppState::new();

        edit.handle_key_stroke(KeyCode::Delete, &mut state);

        assert_eq!(edit.lines(), &vec!["".to_string()]);
        assert_eq!(edit.current_line(), 0);
        assert_eq!(edit.cursor_index_in_line(), 0);
    }

    #[test]
    fn handle_delete_key_middle_of_line() {
        let mut edit = TextPanel::default();
        edit.set_text("abc");
        edit.set_cursor_index(1);

        let mut state = AppState::new();

        edit.handle_key_stroke(KeyCode::Delete, &mut state);

        assert_eq!(edit.lines(), &vec!["ac".to_string()]);
        assert_eq!(edit.current_line(), 0);
        assert_eq!(edit.cursor_index_in_line(), 1);
    }

    #[test]
    fn handle_delete_key_on_empty_line() {
        let mut edit = TextPanel::default();
        edit.set_text("abc\n\ndef");
        edit.set_current_line(1);
        edit.set_cursor_index(0);

        let mut state = AppState::new();

        edit.handle_key_stroke(KeyCode::Delete, &mut state);

        assert_eq!(edit.lines(), &vec!["abc".to_string(), "def".to_string()]);
        assert_eq!(edit.current_line(), 1);
        assert_eq!(edit.cursor_index_in_line(), 0);
    }

    #[test]
    fn handle_delete_key_end_of_line() {
        let mut edit = TextPanel::default();
        edit.set_text("abc\ndef");
        edit.set_current_line(0);
        edit.set_cursor_index(3);

        let mut state = AppState::new();

        edit.handle_key_stroke(KeyCode::Delete, &mut state);

        assert_eq!(edit.lines(), &vec!["abcdef".to_string()]);
        assert_eq!(edit.current_line(), 0);
        assert_eq!(edit.cursor_index_in_line(), 3);
    }

    #[test]
    fn scroll_down_one() {
        let mut edit = TextPanel::default();
        edit.set_text(
            (100..200)
                .map(|i| i.to_string())
                .collect::<Vec<String>>()
                .join("\n"),
        );
        edit.set_scroll_y(95);

        let mut state = AppState::new();

        edit.scroll_down_one(KeyCode::Null, &mut state);

        assert_eq!(edit.scroll_y(), 96);
    }

    #[test]
    fn scroll_down_past_text() {
        let mut edit = TextPanel::default();
        edit.set_text(
            (100..200)
                .map(|i| i.to_string())
                .collect::<Vec<String>>()
                .join("\n"),
        );
        edit.set_scroll_y(95);

        let mut state = AppState::new();

        edit.scroll_down_ten(KeyCode::Null, &mut state);

        assert_eq!(edit.scroll_y(), 100);
    }

    #[test]
    fn scroll_up_one() {
        let mut edit = TextPanel::default();
        let mut state = AppState::new();
        edit.set_scroll_y(6);

        edit.scroll_up_one(KeyCode::Null, &mut state);

        assert_eq!(edit.scroll_y(), 5);
    }

    #[test]
    fn scroll_up_one_at_zero() {
        let mut edit = TextPanel::default();
        let mut state = AppState::new();

        edit.scroll_up_one(KeyCode::Null, &mut state);

        assert_eq!(edit.scroll_y(), 0);
    }

    #[test]
    fn next_character() {
        let mut edit = TextPanel::default();
        edit.set_text(
            (100..200)
                .map(|i| i.to_string())
                .collect::<Vec<String>>()
                .join("\n"),
        );
        edit.set_current_line(2);
        edit.set_cursor_index(2);
        let mut state = AppState::new();

        edit.move_to_next_character(KeyCode::Null, &mut state);

        assert_eq!(edit.cursor_index_in_line(), 3);
    }

    #[test]
    fn next_character_to_next_line() {
        let mut edit = TextPanel::default();
        edit.set_text(
            (100..200)
                .map(|i| i.to_string())
                .collect::<Vec<String>>()
                .join("\n"),
        );
        edit.set_current_line(2);
        edit.set_cursor_index(2);
        let mut state = AppState::new();

        edit.move_to_next_character(KeyCode::Null, &mut state);
        assert_eq!(edit.cursor_index_in_line(), 3);

        edit.move_to_next_character(KeyCode::Null, &mut state);
        assert_eq!(edit.cursor_index_in_line(), 0);
        assert_eq!(edit.current_line(), 3);
    }

    #[test]
    fn previous_character() {
        let mut edit = TextPanel::default();
        edit.set_text(
            (100..200)
                .map(|i| i.to_string())
                .collect::<Vec<String>>()
                .join("\n"),
        );
        edit.set_current_line(2);
        edit.set_cursor_index(2);
        let mut state = AppState::new();

        edit.move_to_previous_character(KeyCode::Null, &mut state);

        assert_eq!(edit.cursor_index_in_line(), 1);
    }

    #[test]
    fn previous_character_to_previous_line() {
        let mut edit = TextPanel::default();
        edit.set_text(
            (100..200)
                .map(|i| i.to_string())
                .collect::<Vec<String>>()
                .join("\n"),
        );
        edit.set_current_line(2);
        edit.set_cursor_index(2);
        let mut state = AppState::new();

        edit.move_to_previous_character(KeyCode::Null, &mut state);
        assert_eq!(edit.cursor_index_in_line(), 1);

        edit.move_to_previous_character(KeyCode::Null, &mut state);
        assert_eq!(edit.cursor_index_in_line(), 0);

        edit.move_to_previous_character(KeyCode::Null, &mut state);
        assert_eq!(edit.cursor_index_in_line(), 3);
        assert_eq!(edit.current_line(), 1);
    }

    #[test]
    fn previous_character_at_zero() {
        let mut edit = TextPanel::default();
        edit.set_text(
            (100..200)
                .map(|i| i.to_string())
                .collect::<Vec<String>>()
                .join("\n"),
        );
        let mut state = AppState::new();

        edit.move_to_previous_character(KeyCode::Null, &mut state);

        assert_eq!(edit.cursor_index_in_line(), 0);
        assert_eq!(edit.current_line(), 0);
    }

    #[test]
    fn next_line() {
        let mut edit = TextPanel::default();
        edit.set_text("12345\n12345".to_string());
        edit.set_current_line(0);
        edit.set_cursor_index(4);
        let mut state = AppState::new();

        edit.move_to_next_line(KeyCode::Null, &mut state);

        assert_eq!(edit.cursor_index_in_line(), 4);
        assert_eq!(edit.current_line(), 1);
    }

    #[test]
    fn next_line_no_line() {
        let mut edit = TextPanel::default();
        edit.set_text("12345\n12345".to_string());
        edit.set_current_line(1);
        edit.set_cursor_index(4);
        let mut state = AppState::new();

        edit.move_to_next_line(KeyCode::Null, &mut state);

        assert_eq!(edit.cursor_index_in_line(), 4);
        assert_eq!(edit.current_line(), 1);
    }

    #[test]
    fn next_line_longer_than_next() {
        let mut edit = TextPanel::default();
        edit.set_text("1234567890\n12345\n1234567890".to_string());
        edit.set_current_line(0);
        edit.set_cursor_index(9);
        let mut state = AppState::new();

        edit.move_to_next_line(KeyCode::Null, &mut state);

        assert_eq!(edit.cursor_index_in_line(), 5);
        assert_eq!(edit.current_line(), 1);
    }

    #[test]
    fn next_line_that_is_last_line() {
        let mut edit = TextPanel::default();
        edit.set_text("1234567890\n12345\n1234567890".to_string());
        edit.set_current_line(1);
        edit.set_cursor_index(4);
        let mut state = AppState::new();

        edit.move_to_next_line(KeyCode::Null, &mut state);

        assert_eq!(edit.cursor_index_in_line(), 4);
        assert_eq!(edit.current_line(), 2);
    }

    #[test]
    fn previous_line() {
        let mut edit = TextPanel::default();
        edit.set_text("1234567890\n1234567890".to_string());
        edit.set_current_line(1);
        edit.set_cursor_index(4);
        let mut state = AppState::new();

        edit.move_to_previous_line(KeyCode::Null, &mut state);

        assert_eq!(edit.cursor_index_in_line(), 4);
        assert_eq!(edit.current_line(), 0);
    }

    #[test]
    fn next_line_longer_than_previous() {
        let mut edit = TextPanel::default();
        edit.set_text("12345\n1234567890".to_string());
        edit.set_current_line(1);
        edit.set_cursor_index(9);
        let mut state = AppState::new();

        edit.move_to_previous_line(KeyCode::Null, &mut state);

        assert_eq!(edit.cursor_index_in_line(), 5);
        assert_eq!(edit.current_line(), 0);
    }
}
