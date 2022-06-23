use std::env;
use std::path::{Component, PathBuf};

use crate::autocomplete::{AutoCompleter, Completion};

pub struct FileAutoCompleter {}

impl FileAutoCompleter {
    pub fn new() -> Self {
        Self {}
    }
}

impl AutoCompleter for FileAutoCompleter {
    fn get_options(&self, s: &str) -> Vec<Completion> {
        let mut path_selection = env::current_dir().unwrap_or(PathBuf::new());

        // push manually, to current dir
        let p = PathBuf::from(s);
        for c in p.components() {
            match c {
                // unix only
                Component::RootDir => path_selection.push(std::path::MAIN_SEPARATOR.to_string()),
                // windows only
                Component::Prefix(p) => path_selection.push(p.as_os_str().to_string_lossy().to_string()),
                Component::CurDir => (),
                Component::ParentDir => {
                    path_selection.pop();
                },
                Component::Normal(s) => match s.to_string_lossy().to_string().as_str() {
                    "~" => {
                        // replaces entire path, since home dir is expected to be absolute
                        // home dir in rust std is deprecated, handle manually here
                        // check $HOME var
                        // if not there, replace with root
                        path_selection = PathBuf::from(match env::var("HOME") {
                            Err(_) => "/".to_string(),
                            Ok(home) => format!("{}/", home)
                        });
                    }
                    s => path_selection.push(s)
                }
            }
        }

        // get directory
        // get file name
        // list directory
        // perform match for each item in directory against file name
        let (current_input, parent) = match (path_selection.file_name(), path_selection.parent()) {
            (None, None) => {
                // currently only getting here when inputting root directory
                // i.e. s == "/"
                // manually make
                (String::new(), path_selection)
            }
            (Some(file_name), Some(parent)) => {
                if s.is_empty() || s.ends_with(std::path::MAIN_SEPARATOR) {
                    // without this check
                    // we would list the current directory name when initially typing or remain in current directory
                    // instead of listing contents of the directory
                    (String::new(), path_selection)
                } else {
                    (
                        file_name.to_string_lossy().to_string(),
                        parent.to_path_buf(),
                    )
                }
            }
            // unknown how we get here, just return no options for now
            _ => return vec![],
        };

        match parent.read_dir() {
            Ok(dir) => {
                let mut options = vec![];

                for d in dir {
                    if let Ok(entry) = d {
                        let entry_name = entry.file_name().to_string_lossy().to_string();
                        if entry_name.starts_with(current_input.as_str()) {
                            let remaining = String::from(&entry_name[current_input.len()..]);
                            options.push(Completion::new(entry_name, remaining));
                        }
                    }
                }

                options
            }
            Err(_) => vec![],
        }
    }
}
