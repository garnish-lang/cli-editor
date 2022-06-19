mod panels;

pub use panels::PanelAutoCompleter;

pub trait AutoCompleter {
    fn get_options(&self, s: &str) -> Vec<String>;
}