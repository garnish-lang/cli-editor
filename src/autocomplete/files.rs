use crate::autocomplete::AutoCompleter;
use std::env;
use std::path::PathBuf;

pub struct FileAutoCompleter {

}

impl FileAutoCompleter {
    pub fn new() -> Self {
        Self {}
    }
}

impl AutoCompleter for FileAutoCompleter {
    fn get_options(&self, s: &str) -> Vec<String> {
        let mut abs_path = env::current_dir().unwrap_or(PathBuf::new());
        abs_path.push(s);

        // get directory
        // get file name
        // list directory
        // perform match for each item in directory agains file name
        match (abs_path.file_name(), abs_path.parent()) {
            (Some(file_name), Some(parent)) => {
                let (file_name, parent) = if s.is_empty() || s.ends_with('/') {
                    // without this check
                    // we would list the current directory name when initially typing or remain in current directory
                    // instead of listing contents of the directory
                    (String::new(), abs_path)
                } else {
                    (file_name.to_string_lossy().to_string(), parent.to_path_buf())
                };

                match parent.read_dir() {
                    Ok(dir) => {
                        let mut options = vec![];

                        for d in dir {
                            if let Ok(entry) = d {
                                let entry_name = entry.file_name().to_string_lossy().to_string();
                                if entry_name.starts_with(file_name.as_str()) {
                                    options.push(entry_name);
                                }
                            }
                        }

                        options
                    }
                    Err(_) => vec![]
                }
            }
            _ => vec![]
        }
    }
}