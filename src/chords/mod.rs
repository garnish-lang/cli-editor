use std::collections::HashMap;
use std::fmt::{Debug, Formatter, Write};

use crossterm::event::{KeyCode, KeyModifiers};

use crate::splits::{split_horizontal, split_vertical};
use crate::AppState;

pub type ChordAction = fn(&mut AppState, KeyCode);

#[derive(Clone)]
pub enum KeyChord {
    Node(
        HashMap<KeyCode, KeyChord>,
        KeyModifiers,
        Option<ChordAction>,
    ),
    Command(ChordAction),
}

impl Debug for KeyChord {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            match self {
                KeyChord::Node(children, mods, action) => {
                    format!(
                        "KeyChord Node: mods {:?} has action {} children {:?}",
                        mods,
                        action.is_some(),
                        children
                    )
                }
                KeyChord::Command(_) => format!("KeyChord Command"),
            }
            .as_str(),
        )
    }
}

pub struct Chords<'a> {
    pub chord_map: HashMap<KeyCode, KeyChord>,
    root: KeyChord,
    pub current_chord: Option<&'a KeyChord>,
}

impl Chords<'_> {
    pub fn new() -> Self {
        Chords {
            root: KeyChord::Node(HashMap::new(), KeyModifiers::empty(), None),
            chord_map: HashMap::new(),
            current_chord: None,
        }
    }

    pub fn builder() -> KeyChordBuilder {
        KeyChordBuilder::new()
    }

    pub fn insert(&mut self, build: fn(KeyChordBuilder) -> KeyChordBuilder) -> Result<(), String> {
        let builder = build(KeyChordBuilder::new());
        let mut current_node = &mut self.root;

        // chain insert all but the last
        for node in builder.nodes.iter().take(builder.nodes.len() - 1) {
            match current_node {
                KeyChord::Node(children, _, _) => {
                    children.insert(
                        node.code,
                        KeyChord::Node(HashMap::new(), node.mods, node.action),
                    );
                    current_node = match children.get_mut(&node.code) {
                        Some(v) => v,
                        _ => unreachable!(),
                    };
                }
                KeyChord::Command(_) => unimplemented!(),
            }
        }

        // last node should be a command
        // insert into current
        let last = &builder.nodes[builder.nodes.len() - 1];
        match current_node {
            KeyChord::Node(children, _, _) => {
                children.insert(last.code, KeyChord::Command(builder.action));
            }
            // should've been validate in first loop
            KeyChord::Command(_) => unreachable!()
        }

        Ok(())
    }
}

impl Chords<'_> {
    pub fn global_chords() -> Self {
        // setup chord commands
        let mut chord_map = HashMap::new();
        chord_map.insert(
            KeyCode::Char('s'),
            KeyChord::Node(
                {
                    let mut h = HashMap::new();
                    h.insert(KeyCode::Char('h'), KeyChord::Command(split_horizontal));
                    h.insert(KeyCode::Char('v'), KeyChord::Command(split_vertical));
                    h
                },
                KeyModifiers::empty(),
                None,
            ),
        );

        chord_map.insert(
            KeyCode::Char('a'),
            KeyChord::Node(
                {
                    let mut h = HashMap::new();
                    h.insert(KeyCode::Null, KeyChord::Command(AppState::select_panel));
                    h
                },
                KeyModifiers::empty(),
                Some(AppState::set_selecting_panel),
            ),
        );

        Chords {
            root: KeyChord::Node(HashMap::new(), KeyModifiers::empty(), None),
            chord_map,
            current_chord: None,
        }
    }
}

pub struct KeyChordNode {
    code: KeyCode,
    mods: KeyModifiers,
    action: Option<ChordAction>,
}

impl KeyChordNode {
    pub fn mods(mut self, mods: KeyModifiers) -> Self {
        self.mods = mods;
        self
    }

    pub fn action(mut self, action: ChordAction) -> Self {
        self.action = Some(action);
        self
    }
}

pub fn key(key: char) -> KeyChordNode {
    KeyChordNode {
        code: KeyCode::Char(key),
        mods: KeyModifiers::empty(),
        action: None,
    }
}

pub fn code(code: KeyCode) -> KeyChordNode {
    KeyChordNode {
        code,
        mods: KeyModifiers::empty(),
        action: None,
    }
}

fn default_action(_: &mut AppState, _: KeyCode) {}

pub struct KeyChordBuilder {
    nodes: Vec<KeyChordNode>,
    action: ChordAction,
}

impl KeyChordBuilder {
    fn new() -> Self {
        KeyChordBuilder {
            nodes: vec![],
            action: default_action,
        }
    }

    pub fn keys(mut self, keys: &str) -> Self {
        for c in keys.chars() {
            self.nodes.push(key(c));
        }

        self
    }

    pub fn node(mut self, c: KeyChordNode) -> Self {
        self.nodes.push(c.into());
        self
    }

    pub fn action(mut self, action: ChordAction) -> Self {
        self.action = action;
        self
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyCode;

    use crate::chords::{code, default_action, key, KeyChordNode};
    use crate::{AppState, Chords, KeyChord};

    fn no_op(state: &mut AppState, _: KeyCode) {
        state.active_panel = 100;
    }

    #[test]
    fn insert_basic() {
        let mut chords = Chords::new();
        chords
            .insert(|b| b.node(key('a')).node(key('b')).node(key('c')).action(no_op))
            .unwrap();

        match chords.root {
            KeyChord::Node(children, _, _) => match children.get(&KeyCode::Char('a')).unwrap() {
                KeyChord::Node(children, _, _) => {
                    match children.get(&KeyCode::Char('b')).unwrap() {
                        KeyChord::Node(children, _, _) => {
                            match children.get(&KeyCode::Char('c')).unwrap() {
                                KeyChord::Command(action) => {
                                    let mut state = AppState::new();
                                    action(&mut state, KeyCode::Null);

                                    assert_eq!(state.active_panel, 100, "State not changed");
                                }
                                _ => panic!("'c' node is not a command"),
                            }
                        }
                        _ => panic!("'b' node is not a node"),
                    }
                }
                _ => panic!("'a' node is not a node"),
            },
            _ => panic!("root not a node"),
        }
    }
}
