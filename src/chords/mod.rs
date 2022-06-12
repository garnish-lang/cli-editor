use std::collections::HashMap;

use crossterm::event::KeyCode;

use crate::AppState;

use crate::splits::{split_horizontal, split_vertical};

#[derive(Clone)]
pub enum KeyChord {
    Node(KeyCode, HashMap<KeyCode, KeyChord>),
    Command(fn(&mut AppState)),
}

pub struct Chords {
    pub chord_map: HashMap<KeyCode, KeyChord>,
    pub current_chord: Option<KeyChord>,
}

impl Chords {
    pub fn global_chords() -> Self {
        // setup chord commands
        let mut chord_map = HashMap::new();
        chord_map.insert(
            KeyCode::Char('s'),
            KeyChord::Node(KeyCode::Char('s'), {
                let mut h = HashMap::new();
                h.insert(KeyCode::Char('h'), KeyChord::Command(split_horizontal));
                h.insert(KeyCode::Char('v'), KeyChord::Command(split_vertical));
                h
            }),
        );

        chord_map.insert(
            KeyCode::Char('a'),
            KeyChord::Node(KeyCode::Char('s'), {
                let mut h = HashMap::new();
                h.insert(KeyCode::Null, KeyChord::Command(split_horizontal));
                h
            }),
        );

        Chords {
            chord_map,
            current_chord: None,
        }
    }
}
