use std::collections::{HashMap, HashSet};
use std::io;
use std::io::Stdout;

use crossterm::event::{
    read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
};
use crossterm::execute;
use crossterm::style::Print;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use tui::backend::CrosstermBackend;
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Style};
use tui::text::{Span, Text};
use tui::widgets::{Block, Borders, Paragraph, Wrap};
use tui::{Frame, Terminal};

trait Panel {
    fn make_widget(&self, frame: &mut EditorFrame, rect: Rect);
    fn get_cursor(&self, rect: &Rect) -> (u16, u16);
    fn get_id(&self) -> char;
    fn set_id(&mut self, id: char);
    fn receive_key(&mut self, event: KeyEvent) -> bool;
    fn set_active(&mut self);
}

struct TextEditPanel {
    id: char,
    min_x: u16,
    min_y: u16,
    cursor_x: u16,
    cursor_y: u16,
    text: String,
}

impl TextEditPanel {
    fn new() -> Self {
        TextEditPanel {
            id: '\0',
            cursor_x: 1,
            cursor_y: 1,
            min_x: 1,
            min_y: 1,
            text: String::new(),
        }
    }
}

impl Panel for TextEditPanel {
    fn make_widget(&self, frame: &mut EditorFrame, rect: Rect) {
        let para_text = Text::from(self.text.clone());
        let para_block = Block::default().title("Block").borders(Borders::ALL);
        let para = Paragraph::new(para_text)
            .block(para_block)
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true });

        frame.render_widget(para, rect);
    }

    fn get_cursor(&self, rect: &Rect) -> (u16, u16) {
        (rect.x + self.cursor_x, rect.y + self.cursor_y)
    }

    fn get_id(&self) -> char {
        self.id
    }

    fn set_id(&mut self, id: char) {
        self.id = id;
    }

    fn receive_key(&mut self, event: KeyEvent) -> bool {
        match event.code {
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
            _ => return false,
        }

        true
    }

    fn set_active(&mut self) {
        todo!()
    }
}

struct PromptPanel {
    id: char,
    min_x: u16,
    min_y: u16,
    cursor_x: u16,
    cursor_y: u16,
    text: String,
}

impl PromptPanel {
    fn new() -> Self {
        PromptPanel {
            id: '\0',
            cursor_x: 1,
            cursor_y: 1,
            min_x: 1,
            min_y: 1,
            text: String::new(),
        }
    }
}

impl Panel for PromptPanel {
    fn make_widget(&self, frame: &mut EditorFrame, rect: Rect) {
        let para_text = Span::from(self.text.clone());
        let para_block = Block::default().title("Prompt").borders(Borders::ALL);
        let para = Paragraph::new(para_text)
            .block(para_block)
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true });

        frame.render_widget(para, rect);
    }

    fn get_cursor(&self, rect: &Rect) -> (u16, u16) {
        (rect.x + self.cursor_x, rect.y + self.cursor_y)
    }

    fn get_id(&self) -> char {
        self.id
    }

    fn set_id(&mut self, id: char) {
        self.id = id;
    }

    fn receive_key(&mut self, event: KeyEvent) -> bool {
        match event.code {
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
                // perform action
            }
            KeyCode::Char(c) => {
                self.cursor_x += 1;
                self.text.push(c);
            }
            _ => return false,
        }

        true
    }

    fn set_active(&mut self) {
        todo!()
    }
}

struct PanelSplit {
    direction: Direction,
    panels: Vec<UserSplits>,
}

impl PanelSplit {
    fn new(direction: Direction, panels: Vec<UserSplits>) -> Self {
        PanelSplit { direction, panels }
    }
}

enum UserSplits {
    Split(usize),
    Panel(usize),
}

enum KeyChord {
    Part(char, HashMap<char, KeyChord>),
    Command(fn(usize, &mut Vec<(usize, Box<dyn Panel>)>, &mut Vec<PanelSplit>)),
}

const PROMPT_HEIGHT: u16 = 3;

type EditorFrame<'a> = Frame<'a, CrosstermBackend<Stdout>>;

fn split(
    active_panel_index: usize,
    direction: Direction,
    panels: &mut Vec<(usize, Box<dyn Panel>)>,
    splits: &mut Vec<PanelSplit>,
) {
    match panels.get(active_panel_index) {
        None => {
            panic!("active panel not found")
        }
        Some((split_i, active_panel)) => {
            let split_i = *split_i;
            let active_panel_id = active_panel.get_id();
            let split_count = splits.len();
            let new_split = match splits.get_mut(split_i) {
                None => {
                    panic!("split not found")
                }
                Some(split) => {
                    // create split
                    let new_split_index = split_count;

                    // create panel
                    let new_panel_index = panels.len();
                    let mut p = TextEditPanel::new();
                    p.set_id(first_available_id(panels));
                    panels.push((new_split_index, Box::new(p)));

                    // update active panel split index
                    let mut child_index = 0;
                    for (i, (_, child)) in panels.iter().enumerate() {
                        if child.get_id() == active_panel_id {
                            child_index = i;
                            break;
                        }
                    }

                    panels[child_index].0 = new_split_index;

                    let new_panel_split = PanelSplit::new(
                        direction,
                        vec![
                            UserSplits::Panel(active_panel_index),
                            UserSplits::Panel(new_panel_index),
                        ],
                    );

                    // find child index for active panel
                    let mut child_index = 0;
                    for (i, child) in split.panels.iter().enumerate() {
                        match child {
                            UserSplits::Split(_) => (),
                            UserSplits::Panel(addr) => {
                                match panels.get(*addr) {
                                    None => (), // error?
                                    Some((_, p)) => {
                                        if p.get_id() == active_panel_id {
                                            child_index = i;
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    split.panels[child_index] = UserSplits::Split(new_split_index);

                    new_panel_split
                }
            };

            splits.push(new_split);
        }
    }
}

fn split_horizontal(
    active_panel_index: usize,
    panels: &mut Vec<(usize, Box<dyn Panel>)>,
    splits: &mut Vec<PanelSplit>,
) {
    split(active_panel_index, Direction::Horizontal, panels, splits)
}

fn split_vertical(
    active_panel_index: usize,
    panels: &mut Vec<(usize, Box<dyn Panel>)>,
    splits: &mut Vec<PanelSplit>,
) {
    split(active_panel_index, Direction::Vertical, panels, splits)
}

fn render_split(
    split: &PanelSplit,
    active_panel: usize,
    splits: &Vec<PanelSplit>,
    panels: &Vec<(usize, Box<dyn Panel>)>,
    frame: &mut EditorFrame,
    chunk: Rect,
) {
    // calculate child width
    let total = match split.direction {
        Direction::Horizontal => chunk.width,
        Direction::Vertical => chunk.height,
    };

    let lengths = match split.panels.len() {
        0 => vec![], // error?
        1 => vec![Constraint::Length(total)],
        n => {
            let part_size = total / n as u16;
            let mut remaining = total;
            let mut lengths = vec![];

            // loop for all but last item n - 1, range is exclusive on end
            // and set length to part size
            // subtract from remaining which will be last item's lengths
            for _ in 0..(n - 1) {
                remaining -= part_size;
                lengths.push(Constraint::Length(part_size));
            }

            lengths.push(Constraint::Length(remaining));

            lengths
        }
    };

    let chunks = Layout::default()
        .direction(split.direction.clone())
        .constraints(lengths)
        .split(chunk);

    // loop through children and render
    for (child, chunk) in split.panels.iter().zip(chunks) {
        match child {
            UserSplits::Panel(panel_i) => match panels.get(*panel_i) {
                None => (), // error
                Some((_, panel)) => {
                    if *panel_i == active_panel {
                        let (x, y) = panel.get_cursor(&chunk);
                        frame.set_cursor(x, y);
                    }

                    panel.make_widget(frame, chunk);
                }
            },
            UserSplits::Split(split_index) => {
                match splits.get(*split_index) {
                    None => (), // error
                    Some(child_split) => {
                        render_split(child_split, active_panel, splits, panels, frame, chunk)
                    }
                }
            }
        }
    }
}

fn first_available_id(panels: &Vec<(usize, Box<dyn Panel>)>) -> char {
    let mut current = HashSet::new();

    for (_, panel) in panels {
        current.insert(panel.get_id());
    }

    let options = ('a'..'z').chain('A'..'Z');

    let mut id = '\0';
    for c in options {
        if !current.contains(&c) {
            id = c;
            break;
        }
    }

    id
}

fn main() -> Result<(), io::Error> {
    enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // setup chord commands
    let mut chord_map = HashMap::new();
    chord_map.insert(
        's',
        KeyChord::Part('s', {
            let mut h = HashMap::new();
            h.insert('h', KeyChord::Command(split_horizontal));
            h.insert('v', KeyChord::Command(split_vertical));
            h
        }),
    );
    let mut current_chord: Option<&KeyChord> = None;

    let mut splits: Vec<PanelSplit> = vec![PanelSplit::new(
        Direction::Horizontal,
        vec![UserSplits::Panel(0)],
    )];

    let mut text_panel = TextEditPanel::new();
    text_panel.set_id('a');

    let mut panels: Vec<(usize, Box<dyn Panel>)> = vec![(0, Box::new(text_panel))];
    let mut prompt_panel = PromptPanel::new();
    prompt_panel.set_id('$');
    let active_panel = 0;

    loop {
        terminal.draw(|frame| {
            let size = frame.size();

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(PROMPT_HEIGHT),
                    Constraint::Length(size.height - PROMPT_HEIGHT),
                ])
                .split(size);

            let (prompt_chunk, user_chunk) = (chunks[0], chunks[1]);

            prompt_panel.make_widget(frame, prompt_chunk);

            render_split(
                &splits[0],
                active_panel,
                &splits,
                &panels,
                frame,
                user_chunk,
            )
        })?;

        match read()? {
            Event::Key(event) => {
                // check if we're in a chord right now
                // if not, check if we're going to start a chord
                // then finally defer to non-chord commands
                match (&current_chord, event.code) {
                    // soft error, just reset
                    // command should've been executed, before being set as current
                    (Some(KeyChord::Command(_)), _) => current_chord = None,
                    // chords can only be formed of chars
                    (Some(KeyChord::Part(_, children)), KeyCode::Char(c)) => {
                        match children.get(&c) {
                            None => current_chord = None, // end chord
                            Some(KeyChord::Command(f)) => {
                                // end of chord, execute function
                                current_chord = None;
                                f(active_panel, &mut panels, &mut splits)
                            }
                            Some(chord) => {
                                // set this chord as current chord
                                current_chord = Some(chord);
                            }
                        }
                    }
                    // in chord but not a char, exit chord
                    (Some(KeyChord::Part(_, _)), _) => {
                        // end chord
                        current_chord = None;
                    }
                    // not in chord, check other commands
                    (None, code) => {
                        // not in chord, but could start one
                        if event.modifiers.contains(KeyModifiers::CONTROL) {
                            // CTRL means a global command including chords
                            // chords without CONTROL will be deferred to active panel
                            match code {
                                KeyCode::Char(c) => {
                                    match chord_map.get(&c) {
                                        Some(chord) => {
                                            current_chord = Some(chord);
                                            continue; // revisit, might not always be last part of loop
                                        }
                                        None => (),
                                    }
                                }
                                _ => (), // not an operation, ignore
                            }
                        } else {
                            // defer to active panel
                            match panels.get_mut(active_panel) {
                                Some((_, panel)) => {
                                    if !panel.receive_key(event) {
                                        match event.code {
                                            KeyCode::Esc => break,
                                            _ => (),
                                        }
                                    }
                                }
                                None => (),
                            }
                        }
                    }
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
