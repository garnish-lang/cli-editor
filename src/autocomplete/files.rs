use std::env;
use std::path::PathBuf;

use crate::autocomplete::{AutoCompleter, Completion};

pub struct FileAutoCompleter {}

impl FileAutoCompleter {
    pub fn new() -> Self {
        Self {}
    }
}

impl AutoCompleter for FileAutoCompleter {
    fn get_options(&self, s: &str) -> Vec<Completion> {
        let mut abs_path = env::current_dir().unwrap_or(PathBuf::new());
        abs_path.push(s);

        // get directory
        // get file name
        // list directory
        // perform match for each item in directory against file name
        let (current_input, parent) = match (abs_path.file_name(), abs_path.parent()) {
            (None, None) => {
                // currently only getting here when inputting root directory
                // i.e. s == "/"
                // manually make
                (String::new(), abs_path)
            }
            (Some(file_name), Some(parent)) => {
                if s.is_empty() || s.ends_with(std::path::MAIN_SEPARATOR) {
                    // without this check
                    // we would list the current directory name when initially typing or remain in current directory
                    // instead of listing contents of the directory
                    (String::new(), abs_path)
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
