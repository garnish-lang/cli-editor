use std::collections::HashMap;

use crossterm::event::KeyCode;

use crate::AppState;

use crate::splits::{split_horizontal, split_vertical};

pub type ChordAction = fn(&mut AppState, KeyCode);

#[derive(Clone)]
pub enum KeyChord {
    Node(HashMap<KeyCode, KeyChord>, Option<ChordAction>),
    Command(ChordAction),
}

pub struct Chords<'a> {
    pub chord_map: HashMap<KeyCode, KeyChord>,
    pub current_chord: Option<&'a KeyChord>,
}

impl Chords<'_> {
    pub fn global_chords() -> Self {
        // setup chord commands
        let mut chord_map = HashMap::new();
        chord_map.insert(
            KeyCode::Char('s'),
            KeyChord::Node({
                let mut h = HashMap::new();
                h.insert(KeyCode::Char('h'), KeyChord::Command(split_horizontal));
                h.insert(KeyCode::Char('v'), KeyChord::Command(split_vertical));
                h
            }, None),
        );

        chord_map.insert(
            KeyCode::Char('a'),
            KeyChord::Node({
                let mut h = HashMap::new();
                h.insert(KeyCode::Null, KeyChord::Command(AppState::select_panel));
                h
            }, Some(AppState::set_selecting_panel)),
        );

        Chords {
            chord_map,
            current_chord: None,
        }
    }
}