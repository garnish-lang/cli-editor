use crate::autocomplete::AutoCompleter;
use crate::panels::{EDIT_PANEL_TYPE_ID, MESSAGE_PANEL_TYPE_ID, PanelFactory};

pub struct PanelAutoCompleter {

}

impl PanelAutoCompleter {
    pub fn new() -> Self {
        Self {}
    }

    fn options() -> Vec<&'static str> {
        vec![
            EDIT_PANEL_TYPE_ID,
            MESSAGE_PANEL_TYPE_ID,
        ]
    }
}

impl AutoCompleter for PanelAutoCompleter {
    fn get_options(&self, s: &str) -> Vec<String> {
        PanelAutoCompleter::options().iter().filter(|o| o.starts_with(s)).map(|s| s.to_string()).collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::autocomplete::AutoCompleter;
    use crate::autocomplete::panels::PanelAutoCompleter;
    use crate::panels::PanelFactory;

    #[test]
    fn empty_input_returns_all() {
        let completer = PanelAutoCompleter::new();

        assert_eq!(completer.get_options(""), PanelFactory::options());
    }

    #[test]
    fn finds_match() {
        let completer = PanelAutoCompleter::new();

        assert_eq!(completer.get_options("E"), vec!["Edit"]);
        assert_eq!(completer.get_options("Ed"), vec!["Edit"]);
        assert_eq!(completer.get_options("Edi"), vec!["Edit"]);
        assert_eq!(completer.get_options("Edit"), vec!["Edit"]);
        assert_eq!(completer.get_options("Edits"), Vec::<String>::new());
    }
}
