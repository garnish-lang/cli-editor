use crate::autocomplete::AutoCompleter;
use crate::panels::PanelFactory;

pub struct PanelAutoCompleter {

}

impl PanelAutoCompleter {
    pub fn new() -> Self {
        Self {}
    }
}

impl AutoCompleter for PanelAutoCompleter {
    fn get_options(&self, s: &str) -> Vec<String> {
        PanelFactory::options().iter().filter(|o| o.starts_with(s)).map(|s| s.to_string()).collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::autocomplete::AutoCompleter;
    use crate::autocomplete::panels::PanelAutoCompleter;

    #[test]
    fn finds_match() {
        let completer = PanelAutoCompleter::new();

        assert_eq!(completer.get_options("N"), vec!["Null"]);
        assert_eq!(completer.get_options("Nu"), vec!["Null"]);
        assert_eq!(completer.get_options("Nul"), vec!["Null"]);
        assert_eq!(completer.get_options("Null"), vec!["Null"]);
        assert_eq!(completer.get_options("Nulls"), Vec::<String>::new());
    }
}
