use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::hash::Hash;

use crossterm::event::{KeyCode, KeyModifiers};

pub use manager::Manager;

mod manager;

#[derive(Clone)]
pub enum CommandKey<T> {
    Node(
        KeyCode,
        KeyModifiers,
        HashMap<CommandKeyId, CommandKey<T>>,
        Option<T>,
    ),
    Leaf(KeyCode, KeyModifiers, CommandDetails, T),
}

impl<T> CommandKey<T> {
    fn get_hash(&self) -> CommandKeyId {
        let (c, m) = match self {
            CommandKey::Node(c, m, _, _) => (c, m),
            CommandKey::Leaf(c, m, _, _) => (c, m),
        };

        CommandKeyId::new(*c, *m)
    }
}

impl<T> Debug for CommandKey<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            match self {
                CommandKey::Node(code, mods, children, action) => {
                    format!(
                        "KeyChord Node: code {:?} mods {:?} has action {} children {:?}",
                        code,
                        mods,
                        action.is_some(),
                        children
                    )
                }
                CommandKey::Leaf(code, mods, _, _) => {
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

#[allow(dead_code)]
impl CommandDetails {
    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn description(&self) -> &String {
        &self.description
    }

    pub fn empty() -> Self {
        CommandDetails {
            name: String::new(),
            description: String::new(),
        }
    }

    pub fn new<T1: ToString, T2: ToString>(name: T1, description: T2) -> Self {
        CommandDetails {
            name: name.to_string(),
            description: description.to_string(),
        }
    }

    pub fn split_horizontal() -> Self {
        CommandDetails {
            name: "Split Horizontal".to_string(),
            description: "Split active panel into two panels that are horizontally aligned."
                .to_string(),
        }
    }

    pub fn split_vertical() -> Self {
        CommandDetails {
            name: "Split Vertical".to_string(),
            description: "Split active panel into two panels that are vertically aligned."
                .to_string(),
        }
    }

    pub fn add_panel() -> Self {
        CommandDetails {
            name: "Add Panel".to_string(),
            description: "Add panel to active split.".to_string(),
        }
    }

    pub fn remove_panel() -> Self {
        CommandDetails {
            name: "Remove".to_string(),
            description: "Remove active panel.".to_string(),
        }
    }

    pub fn change_panel_type() -> Self {
        CommandDetails {
            name: "Change Panel Type".to_string(),
            description: "Change type of active panel".to_string(),
        }
    }

    pub fn activate_next_panel() -> Self {
        CommandDetails {
            name: "Next Panel".to_string(),
            description: "Activate next panel".to_string(),
        }
    }

    pub fn activate_previous_panel() -> Self {
        CommandDetails {
            name: "Previous Panel".to_string(),
            description: "Activate previous panel".to_string(),
        }
    }

    pub fn select_panel() -> Self {
        CommandDetails {
            name: "Activate Panel".to_string(),
            description: "Activate a panel by selecting its ID. The IDs will be displayed next to panel titles after first key.".to_string()
        }
    }

    pub fn open_file() -> Self {
        CommandDetails {
            name: "Open File".to_string(),
            description: "Open a file by typing name in input panel.".to_string(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct CommandKeyId {
    code: KeyCode,
    mods: KeyModifiers,
}

#[allow(dead_code)]
impl CommandKeyId {
    pub fn new(code: KeyCode, mods: KeyModifiers) -> Self {
        CommandKeyId { code, mods }
    }

    pub fn new_code(code: KeyCode) -> Self {
        CommandKeyId {
            code,
            mods: KeyModifiers::empty(),
        }
    }

    pub fn code(&self) -> KeyCode {
        self.code
    }

    pub fn mods(&self) -> KeyModifiers {
        self.mods
    }
}

pub struct Commands<T> {
    root: CommandKey<T>,
}

#[allow(dead_code)]
impl<T> Commands<T>
where
    T: Copy,
{
    pub fn new() -> Self {
        Commands {
            root: CommandKey::Node(KeyCode::Null, KeyModifiers::empty(), HashMap::new(), None),
        }
    }

    pub fn builder() -> CommandSequenceBuilder<T> {
        CommandSequenceBuilder::new()
    }

    pub fn insert(
        &mut self,
        build: fn(CommandSequenceBuilder<T>) -> CommandSequenceBuilder<T>,
    ) -> Result<(), String> {
        let builder = build(CommandSequenceBuilder::new());
        let mut current_node = &mut self.root;

        // chain insert all but the last
        for node in builder.nodes.iter().take(builder.nodes.len() - 1) {
            match current_node {
                CommandKey::Node(_, _, children, _) => {
                    let h = CommandKeyId::new(node.code, node.mods);
                    let n = CommandKey::Node(node.code, node.mods, HashMap::new(), node.action);
                    current_node = children.entry(h).or_insert(n)
                }
                CommandKey::Leaf(_, _, _, _) => {
                    return Err("Existing command in sequence.".to_string())
                }
            }
        }

        // last node should be a command
        // insert into current
        let last = &builder.nodes[builder.nodes.len() - 1];
        match current_node {
            CommandKey::Node(_, _, children, _) => {
                // make sure we were given a action
                match builder.action {
                    Some(action) => {
                        let n = CommandKey::Leaf(last.code, last.mods, builder.details, action);
                        children.insert(n.get_hash(), n);
                    }
                    None => return Err("Missing command action.".to_string()),
                }
            }
            // should've been validate in first loop
            CommandKey::Leaf(_, _, _, _) => return Err("Existing command in sequence.".to_string()),
        }

        Ok(())
    }

    pub fn remove(
        &mut self,
        build: fn(CommandSequenceBuilder<T>) -> CommandSequenceBuilder<T>,
    ) -> Result<(), String> {
        let builder = build(CommandSequenceBuilder::new());
        // manual count of nesting
        // drill down and keep track of the lowest node with only 1 child
        let mut index = 0;
        let mut lowest = 0;

        let mut current_node = &self.root;
        for node in &builder.nodes {
            match current_node {
                CommandKey::Node(_, _, children, _) => {
                    let h = CommandKeyId::new(node.code, node.mods);
                    match children.get(&h) {
                        // no child with given sequence, effectively means its already removed
                        // just return
                        None => return Ok(()),
                        Some(c) => current_node = c,
                    };

                    // 1 or fewer children means this entire branch will be removed
                    if children.len() > 1 {
                        lowest = index + 1;
                    }
                    index += 1;
                }
                // end of branch
                CommandKey::Leaf(_, _, _, _) => (),
            }
        }

        // drill down lowest number of times and remove that node from its parent
        let mut current_node = &mut self.root;
        index = 0;

        for node in &builder.nodes {
            match current_node {
                CommandKey::Node(_, _, children, _) => {
                    let h = CommandKeyId::new(node.code, node.mods);
                    // 1 or fewer children means this entire branch will be removed
                    if index == lowest {
                        children.remove(&h);
                        break;
                    }
                    match children.get_mut(&h) {
                        // no child with given sequence, effectively means its already removed
                        // just return
                        None => return Ok(()),
                        Some(c) => current_node = c,
                    }
                    index += 1;
                }
                // end of branch
                CommandKey::Leaf(_, _, _, _) => (),
            }
        }

        Ok(())
    }

    pub fn is_end(&self, progress: &Vec<CommandKeyId>) -> bool {
        let mut current = &self.root;

        for (index, c) in progress.iter().enumerate() {
            match current {
                CommandKey::Node(_, _, children, _) => match children.get(c) {
                    Some(next) => current = next,
                    // not a valid path, always considered to be end
                    None => return true,
                },
                // reached leaf mid way through path, considered end
                CommandKey::Leaf(_, _, _, a) => return true,
            }
        }

        match current {
            CommandKey::Node(..) => false,
            CommandKey::Leaf(..) => true,
        }
    }

    pub fn get(&self, path: &Vec<CommandKeyId>) -> Option<(bool, Option<T>)> {
        self.get_node(path).and_then(|current| {
            Some(match current {
                CommandKey::Node(.., Some(action)) => (false, Some(*action)),
                CommandKey::Node(..) => (false, None),
                CommandKey::Leaf(.., action) => (true, Some(*action)),
            })
        })
    }

    pub fn get_node(&self, path: &Vec<CommandKeyId>) -> Option<&CommandKey<T>> {
        let mut current = &self.root;
        for c in path {
            match current {
                CommandKey::Node(_, _, children, _) => match children.get(c) {
                    Some(next) => current = next,
                    // no direct match
                    // check for catch all Null code, cloning given modifiers
                    None => match children.get(&CommandKeyId::new(KeyCode::Null, c.mods)) {
                        Some(next) => current = next,
                        // current path leads nowhere
                        // return nothing
                        None => return None,
                    },
                },
                CommandKey::Leaf(..) => {
                    // current path goes beyond command
                    // return early with end result
                    break;
                }
            }
        }

        Some(current)
    }
}

#[derive(Clone)]
pub struct CommandKeyBuilder<T> {
    code: KeyCode,
    mods: KeyModifiers,
    action: Option<T>,
}

#[allow(dead_code)]
impl<T> CommandKeyBuilder<T> {
    pub fn mods(mut self, mods: KeyModifiers) -> Self {
        self.mods = mods;
        self
    }

    pub fn action(mut self, action: T) -> Self {
        self.action = Some(action);
        self
    }
}

pub fn alt_key<T>(key: char) -> CommandKeyBuilder<T> {
    CommandKeyBuilder {
        code: KeyCode::Char(key),
        mods: KeyModifiers::ALT,
        action: None,
    }
}

pub fn shift_alt_key<T>(key: char) -> CommandKeyBuilder<T> {
    CommandKeyBuilder {
        code: KeyCode::Char(key),
        mods: KeyModifiers::SHIFT | KeyModifiers::ALT,
        action: None,
    }
}

pub fn ctrl_key<T>(key: char) -> CommandKeyBuilder<T> {
    CommandKeyBuilder {
        code: KeyCode::Char(key),
        mods: KeyModifiers::CONTROL,
        action: None,
    }
}

pub fn ctrl_alt_key<T>(key: char) -> CommandKeyBuilder<T> {
    CommandKeyBuilder {
        code: KeyCode::Char(key),
        mods: KeyModifiers::CONTROL | KeyModifiers::ALT,
        action: None,
    }
}

pub fn key<T>(key: char) -> CommandKeyBuilder<T> {
    CommandKeyBuilder {
        code: KeyCode::Char(key),
        mods: KeyModifiers::empty(),
        action: None,
    }
}

#[allow(dead_code)]
pub fn code<T>(code: KeyCode) -> CommandKeyBuilder<T> {
    CommandKeyBuilder {
        code,
        mods: KeyModifiers::empty(),
        action: None,
    }
}

pub fn catch_all<T>() -> CommandKeyBuilder<T> {
    CommandKeyBuilder {
        code: KeyCode::Null,
        mods: KeyModifiers::empty(),
        action: None,
    }
}

pub fn shift_catch_all<T>() -> CommandKeyBuilder<T> {
    CommandKeyBuilder {
        code: KeyCode::Null,
        mods: KeyModifiers::SHIFT,
        action: None,
    }
}

pub fn alt_catch_all<T>() -> CommandKeyBuilder<T> {
    CommandKeyBuilder {
        code: KeyCode::Null,
        mods: KeyModifiers::ALT,
        action: None,
    }
}

pub struct CommandSequenceBuilder<T> {
    nodes: Vec<CommandKeyBuilder<T>>,
    details: CommandDetails,
    action: Option<T>,
}

#[allow(dead_code)]
impl<T> CommandSequenceBuilder<T> {
    fn new() -> Self {
        CommandSequenceBuilder {
            nodes: vec![],
            details: CommandDetails::empty(),
            action: None,
        }
    }

    pub fn keys(mut self, keys: &str) -> Self {
        for c in keys.chars() {
            self.nodes.push(key(c));
        }

        self
    }

    pub fn node(mut self, c: CommandKeyBuilder<T>) -> Self {
        self.nodes.push(c.into());
        self
    }

    pub fn action(mut self, details: CommandDetails, action: T) -> Self {
        self.details = details;
        self.action = Some(action);
        self
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyModifiers};

    use crate::commands::{code, key, CommandDetails, CommandKey, CommandKeyId};
    use crate::{AppState, Commands};

    fn no_op(state: &mut AppState, _: KeyCode) {
        state.set_active_panel(100)
    }

    type CommandAction = fn(&mut AppState, KeyCode);

    fn assert_sequence(root: &CommandKey<fn(&mut AppState, KeyCode)>, sequence: &[char]) {
        let mut current = root;
        for c in sequence {
            match current {
                CommandKey::Node(_, _, children, _) => {
                    match children.get(&CommandKeyId::new_code(KeyCode::Char(*c))) {
                        Some(n) => current = n,
                        None => panic!("{} not found in children", c),
                    }
                }
                k => panic!("{:?} node is not Node", k),
            }
        }

        match current {
            CommandKey::Leaf(_, _, _, action) => {
                let mut state = AppState::new();
                action(&mut state, KeyCode::Null);

                assert_eq!(state.active_panel(), 100, "State not changed");
            }
            k => panic!("{:?} is not a Command", k),
        }
    }

    #[test]
    fn insert_basic() {
        let mut commands = Commands::<CommandAction>::new();

        commands
            .insert(|b| {
                b.node(key('a'))
                    .node(key('b'))
                    .node(key('c'))
                    .action(CommandDetails::split_horizontal(), no_op)
            })
            .unwrap();

        assert_sequence(&commands.root, &['a', 'b', 'c'])
    }

    #[test]
    fn insert_without_action_is_err() {
        let mut commands = Commands::<CommandAction>::new();

        let result = commands.insert(|b| b.node(key('a')).node(key('b')).node(key('c')));

        assert!(result.is_err());
    }

    #[test]
    fn insert_two_same_start() {
        let mut commands = Commands::<CommandAction>::new();

        commands
            .insert(|b| {
                b.node(key('a'))
                    .node(key('b'))
                    .node(key('c'))
                    .action(CommandDetails::split_horizontal(), no_op)
            })
            .unwrap();

        commands
            .insert(|b| {
                b.node(key('a'))
                    .node(key('b'))
                    .node(key('d'))
                    .action(CommandDetails::split_horizontal(), no_op)
            })
            .unwrap();

        assert_sequence(&commands.root, &['a', 'b', 'c']);
        assert_sequence(&commands.root, &['a', 'b', 'd']);
    }

    #[test]
    fn insert_beyond_existing_command() {
        let mut commands = Commands::<CommandAction>::new();

        commands
            .insert(|b| {
                b.node(key('a'))
                    .node(key('b'))
                    .node(key('c'))
                    .action(CommandDetails::split_horizontal(), no_op)
            })
            .unwrap();

        let result = commands.insert(|b| {
            b.node(key('a'))
                .node(key('b'))
                .node(key('c'))
                .node(key('d'))
                .action(CommandDetails::split_horizontal(), no_op)
        });

        assert_sequence(&commands.root, &['a', 'b', 'c']);
        assert!(result.is_err());
    }

    #[test]
    fn insert_beyond_existing_command_extended() {
        let mut commands = Commands::<CommandAction>::new();

        commands
            .insert(|b| {
                b.node(key('a'))
                    .node(key('b'))
                    .node(key('c'))
                    .action(CommandDetails::split_horizontal(), no_op)
            })
            .unwrap();

        let result = commands.insert(|b| {
            b.node(key('a'))
                .node(key('b'))
                .node(key('c'))
                .node(key('d'))
                .node(key('e'))
                .node(key('f'))
                .action(CommandDetails::split_horizontal(), no_op)
        });

        assert!(result.is_err());
    }

    #[test]
    fn remove() {
        let mut commands = Commands::<CommandAction>::new();

        commands
            .insert(|b| {
                b.node(key('a'))
                    .node(key('b'))
                    .node(key('c'))
                    .action(CommandDetails::split_horizontal(), no_op)
            })
            .unwrap();

        commands
            .remove(|b| {
                b.node(key('a'))
                    .node(key('b'))
                    .node(key('c'))
                    .action(CommandDetails::split_horizontal(), no_op)
            })
            .unwrap();

        match commands.root {
            CommandKey::Node(_, _, children, _) => assert!(children.is_empty()),
            _ => panic!("Not a Node"),
        }
    }

    #[test]
    fn remove_leaves_sibling_branch() {
        let mut commands = Commands::<CommandAction>::new();

        commands
            .insert(|b| {
                b.node(key('a'))
                    .node(key('b'))
                    .node(key('c'))
                    .node(key('d'))
                    .action(CommandDetails::split_horizontal(), no_op)
            })
            .unwrap();

        commands
            .insert(|b| {
                b.node(key('a'))
                    .node(key('b'))
                    .node(key('e'))
                    .node(key('f'))
                    .action(CommandDetails::split_horizontal(), no_op)
            })
            .unwrap();

        commands
            .remove(|b| {
                b.node(key('a'))
                    .node(key('b'))
                    .node(key('c'))
                    .node(key('d'))
                    .action(CommandDetails::split_horizontal(), no_op)
            })
            .unwrap();

        assert_sequence(&commands.root, &['a', 'b', 'e', 'f']);
    }

    #[test]
    fn remove_absent_sequence() {
        let mut commands = Commands::<CommandAction>::new();

        commands
            .insert(|b| {
                b.node(key('a'))
                    .node(key('b'))
                    .node(key('c'))
                    .action(CommandDetails::split_horizontal(), no_op)
            })
            .unwrap();

        commands
            .remove(|b| {
                b.node(key('a'))
                    .node(key('b'))
                    .node(key('d'))
                    .action(CommandDetails::split_horizontal(), no_op)
            })
            .unwrap();

        assert_sequence(&commands.root, &['a', 'b', 'c']);
    }

    fn details(name: String) -> CommandDetails {
        CommandDetails {
            name,
            description: String::new(),
        }
    }

    #[test]
    fn is_end() {
        let mut commands = Commands::<CommandAction>::new();

        commands
            .insert(|b| {
                b.node(key('a'))
                    .node(key('b'))
                    .node(key('c'))
                    .action(details("abc".to_string()), no_op)
            })
            .unwrap();

        commands
            .insert(|b| {
                b.node(key('a'))
                    .node(key('b'))
                    .node(key('d'))
                    .action(details("abd".to_string()), no_op)
            })
            .unwrap();

        commands
            .insert(|b| {
                b.node(key('a'))
                    .node(key('e'))
                    .node(key('f'))
                    .action(details("aef".to_string()), no_op)
            })
            .unwrap();

        assert!(!commands.is_end(&vec![]));

        let mut progress = vec![
            CommandKeyId::new(KeyCode::Char('a'), KeyModifiers::empty()),
            CommandKeyId::new(KeyCode::Char('b'), KeyModifiers::empty()),
        ];

        assert!(!commands.is_end(&progress));

        progress.push(CommandKeyId::new(KeyCode::Char('d'), KeyModifiers::empty()));

        assert!(commands.is_end(&progress));

        progress.push(CommandKeyId::new(KeyCode::Char('e'), KeyModifiers::empty()));

        assert!(commands.is_end(&progress));
    }
}
