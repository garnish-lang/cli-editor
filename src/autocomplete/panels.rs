use crate::autocomplete::{AutoCompleter, Completion};
use crate::panels::{EDIT_PANEL_TYPE_ID, MESSAGE_PANEL_TYPE_ID};

pub struct PanelAutoCompleter {}

impl PanelAutoCompleter {
    pub fn new() -> Self {
        Self {}
    }

    fn options() -> Vec<&'static str> {
        vec![EDIT_PANEL_TYPE_ID, MESSAGE_PANEL_TYPE_ID]
    }
}

impl AutoCompleter for PanelAutoCompleter {
    fn get_options(&self, s: &str) -> Vec<Completion> {
        PanelAutoCompleter::options()
            .iter()
            .filter(|o| o.starts_with(s))
            .map(|o| Completion::new(o.to_string(), String::from(&o[s.len()..])))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::autocomplete::panels::PanelAutoCompleter;
    use crate::autocomplete::{AutoCompleter, Completion};

    #[test]
    fn empty_input_returns_all() {
        let completer = PanelAutoCompleter::new();

        assert_eq!(completer.get_options("").len(), PanelAutoCompleter::options().len());
    }

    #[test]
    fn finds_match() {
        let completer = PanelAutoCompleter::new();

        assert_eq!(completer.get_options("E"), vec![Completion::new("Edit".to_string(), "dit".to_string())]);
        assert_eq!(completer.get_options("Ed"), vec![Completion::new("Edit".to_string(), "it".to_string())]);
        assert_eq!(completer.get_options("Edi"), vec![Completion::new("Edit".to_string(), "t".to_string())]);
        assert_eq!(completer.get_options("Edit"), vec![Completion::new("Edit".to_string(), "".to_string())]);
        assert_eq!(completer.get_options("Edits"), Vec::<Completion>::new());
    }
}
