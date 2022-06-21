mod panels;
mod files;

pub use panels::PanelAutoCompleter;
pub use files::FileAutoCompleter;

pub trait AutoCompleter {
    fn get_options(&self, s: &str) -> Vec<String>;
}