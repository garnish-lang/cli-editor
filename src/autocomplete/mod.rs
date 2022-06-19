mod panels;

pub trait AutoCompleter {
    fn get_options(&self, s: &str) -> Vec<String>;
}