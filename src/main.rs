use std::io;

use crossterm::event::{read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode};
use crossterm::execute;
use crossterm::style::Print;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use tui::backend::CrosstermBackend;
use tui::layout::{Alignment, Constraint, Direction, Layout};
use tui::style::{Color, Style};
use tui::text::Text;
use tui::widgets::{Block, Borders, Paragraph, Wrap};
use tui::Terminal;

fn main() -> Result<(), io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // let mut input = String::new();
    let mut text = String::new();

    let (min_x, min_y) = (1, 1);
    let (mut cursor_x, mut cursor_y) = (min_x, min_y);
    // let mut workspace_size = 25;
    // let mut input_size = 3;
    // let mut input_active = false;
    //
    // let mut active_text = &mut text;

    loop {
        terminal.draw(|f| {
            let size = f.size();

            let layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(size.width)])
                .split(size);

            let para_text = Text::from(text.as_str());
            let para_block = Block::default().title("Block").borders(Borders::ALL);
            let para = Paragraph::new(para_text)
                .block(para_block)
                .style(Style::default().fg(Color::White).bg(Color::Black))
                .alignment(Alignment::Left)
                .wrap(Wrap { trim: true });

            let (start_x, start_y) = (layout[0].x, layout[0].y);

            // + 1 from borders
            f.set_cursor(start_x + cursor_x, start_y + cursor_y);

            f.render_widget(para, layout[0]);
        })?;

        match read()? {
            Event::Key(event) => {
                match event.code {
                    KeyCode::Esc => break,
                    KeyCode::Backspace => {
                        match text.pop() {
                            None => {
                                cursor_x = min_x;
                                cursor_y = min_y;
                            }
                            Some(c) => {
                                match c {
                                    '\n' => {
                                        cursor_y -= 1;
                                        cursor_x = min_x;

                                        // count from back until a newline is reached
                                        for c in text.chars().rev() {
                                            if c == '\n' {
                                                break;
                                            }
                                            cursor_x += 1;
                                        }
                                    }
                                    _ => {
                                        cursor_x -= 1;
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::Delete => {
                        // ??
                    }
                    KeyCode::Enter => {
                        text.push('\n');
                        cursor_y += 1;
                        cursor_x = 1;
                    }
                    KeyCode::Char(c) => {
                        cursor_x += 1;
                        text.push(c);
                    }
                    _ => (), // execute!(terminal.backend_mut(), Print(format!("{:?}", event)))?
                }
            }
            Event::Mouse(_event) => (), // println!("{:?}", event),
            Event::Resize(width, height) => execute!(
                terminal.backend_mut(),
                Print(format!("New size {}x{}", width, height))
            )?,
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
