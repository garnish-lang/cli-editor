use std::collections::HashMap;
use std::fmt::{Debug, Formatter, Write};
use std::hash::{Hash, Hasher};

use crossterm::event::{KeyCode, KeyModifiers};

use crate::splits::{split_horizontal, split_vertical};
use crate::AppState;

pub type ChordAction = fn(&mut AppState, KeyCode);

#[derive(Clone)]
pub enum KeyChord {
    Node(
        KeyCode,
        KeyModifiers,
        HashMap<ChordHash, KeyChord>,
        Option<ChordAction>,
    ),
    Command(KeyCode, KeyModifiers, CommandDetails, ChordAction),
}

impl KeyChord {
    fn get_hash(&self) -> ChordHash {
        let (c, m) = match self {
            KeyChord::Node(c, m, _, _) => (c, m),
            KeyChord::Command(c, m, _, _) => (c, m),
        };

        ChordHash::new(*c, *m)
    }
}

impl Debug for KeyChord {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            match self {
                KeyChord::Node(code, mods, children, action) => {
                    format!(
                        "KeyChord Node: code {:?} mods {:?} has action {} children {:?}",
                        code,
                        mods,
                        action.is_some(),
                        children
                    )
                }
                KeyChord::Command(code, mods, _, _) => {
                    format!("KeyChord Command: code {:?} mods {:?}", code, mods)
                }
            }
            .as_str(),
        )
    }
}

#[derive(Clone)]
pub struct CommandDetails {
    name: String,
    description: String,
}

impl CommandDetails {
    pub fn name(&self) -> String {
        self.name.to_string()
    }

    pub fn description(&self) -> String {
        self.description.to_string()
    }

    fn empty() -> Self {
        CommandDetails {
            name: String::new(),
            description: String::new(),
        }
    }

    fn split_horizontal() -> Self {
        CommandDetails {
            name: "Split Horizontal".to_string(),
            description: "Split active panel into two panels that are horizontally aligned."
                .to_string(),
        }
    }

    fn split_vertical() -> Self {
        CommandDetails {
            name: "Split Vertical".to_string(),
            description: "Split active panel into two panels that are vertically aligned."
                .to_string(),
        }
    }

    fn select_panel() -> Self {
        CommandDetails {
            name: "Activate Panel".to_string(),
            description: "Activate a panel by selecting its ID. The IDs will be displayed next to panel titles after first key.".to_string()
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ChordHash {
    code: KeyCode,
    mods: KeyModifiers,
}

impl ChordHash {
    pub fn new(code: KeyCode, mods: KeyModifiers) -> Self {
        ChordHash { code, mods }
    }

    pub fn new_code(code: KeyCode) -> Self {
        ChordHash {
            code,
            mods: KeyModifiers::empty(),
        }
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
            root: KeyChord::Node(KeyCode::Null, KeyModifiers::empty(), HashMap::new(), None),
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
                KeyChord::Node(_, _, children, _) => {
                    let n = KeyChord::Node(node.code, node.mods, HashMap::new(), node.action);
                    let h = n.get_hash();
                    children.insert(n.get_hash(), n);
                    current_node = match children.get_mut(&h) {
                        Some(v) => v,
                        _ => unreachable!(),
                    };
                }
                KeyChord::Command(_, _, _, _) => unimplemented!(),
            }
        }

        // last node should be a command
        // insert into current
        let last = &builder.nodes[builder.nodes.len() - 1];
        match current_node {
            KeyChord::Node(_, _, children, _) => {
                let n = KeyChord::Command(last.code, last.mods, builder.details, builder.action);
                children.insert(n.get_hash(), n);
            }
            // should've been validate in first loop
            KeyChord::Command(_, _, _, _) => unreachable!(),
        }

        Ok(())
    }
}

impl Chords<'_> {
    pub fn global_chords() -> Self {
        let mut chords = Chords::new();

        chords
            .insert(|b| {
                b.node(key('s'))
                    .node(key('s'))
                    .action(CommandDetails::split_horizontal(), split_horizontal)
            })
            .unwrap();

        chords
            .insert(|b| {
                b.node(key('s'))
                    .node(key('v'))
                    .action(CommandDetails::split_vertical(), split_vertical)
            })
            .unwrap();

        chords
            .insert(|b| {
                b.node(key('a'))
                    .node(code(KeyCode::Null))
                    .action(CommandDetails::select_panel(), AppState::select_panel)
            })
            .unwrap();

        chords
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
    details: CommandDetails,
    action: ChordAction,
}

impl KeyChordBuilder {
    fn new() -> Self {
        KeyChordBuilder {
            nodes: vec![],
            details: CommandDetails::empty(),
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

    pub fn action(mut self, details: CommandDetails, action: ChordAction) -> Self {
        self.details = details;
        self.action = action;
        self
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyCode;

    use crate::chords::{code, default_action, key, ChordHash, CommandDetails, KeyChordNode};
    use crate::{AppState, Chords, KeyChord};

    fn no_op(state: &mut AppState, _: KeyCode) {
        state.active_panel = 100;
    }

    fn assert_sequence(root: &KeyChord, sequence: &[char]) {
        let mut current = root;
        for c in sequence {
            match current {
                KeyChord::Node(_, _, children, _) => {
                    match children.get(&ChordHash::new_code(KeyCode::Char(*c))) {
                        Some(n) => current = n,
                        None => panic!("{} not found in children", c),
                    }
                }
                k => panic!("{:?} node is not Node", k),
            }
        }

        match current {
            KeyChord::Command(_, _, _, action) => {
                let mut state = AppState::new();
                action(&mut state, KeyCode::Null);

                assert_eq!(state.active_panel, 100, "State not changed");
            }
            k => panic!("{:?} is not a Command", k),
        }
    }

    #[test]
    fn insert_basic() {
        let mut chords = Chords::new();
        chords
            .insert(|b| {
                b.node(key('a'))
                    .node(key('b'))
                    .node(key('c'))
                    .action(CommandDetails::split_horizontal(), no_op)
            })
            .unwrap();

        assert_sequence(&chords.root, &['a', 'b', 'c'])
    }
}
