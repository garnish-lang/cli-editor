pub use files::FileAutoCompleter;
pub use panels::PanelAutoCompleter;

mod files;
mod panels;

pub trait AutoCompleter {
    fn get_options(&self, s: &str) -> Vec<Completion>;
}

pub struct Completion {
    option: String,
    remaining: String,
}

impl Completion {
    pub fn new(option: String, remaining: String) -> Self {
        Self { option, remaining }
    }

    pub fn option(&self) -> &String {
        &self.option
    }

    pub fn remaining(&self) -> &String {
        &self.remaining
    }
}
