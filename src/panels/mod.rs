use crossterm::event::KeyEvent;
use tui::layout::Rect;
use tui::text::Span;
use tui::widgets::Block;

pub use edit::TextEditPanel;
pub use factory::*;
pub use input::InputPanel;
pub use messages::MessagesPanel;
pub use null::NullPanel;

use crate::app::StateChangeRequest;
use crate::{AppState, EditorFrame};

mod edit;
mod factory;
mod input;
mod messages;
mod null;

pub trait Panel {
    fn panel_type(&self) -> &str;
    fn init(&mut self, _state: &mut AppState) {}
    fn make_widget(
        &self,
        _state: &AppState,
        _frame: &mut EditorFrame,
        _rect: Rect,
        _is_active: bool,
        _block: Block,
    ) {
    }
    fn get_cursor(&self) -> (u16, u16) {
        (0, 0)
    }
    fn make_title(&self, _state: &AppState) -> Vec<Span> {
        vec![]
    }
    fn get_length(&self, _state: &AppState) -> u16 {
        0
    }
    fn receive_key(&mut self, _event: KeyEvent, _state: &mut AppState) -> (bool, Vec<StateChangeRequest>) {
        (false, vec![])
    }
    fn receive_input(&mut self, _input: String) -> Vec<StateChangeRequest> {
        vec![]
    }
    fn show(&mut self) {}
    fn hide(&mut self) {}
    fn visible(&self) -> bool {
        true
    }
    fn update(&mut self) -> Vec<StateChangeRequest> {
        vec![]
    }
}

pub struct Panels {
    panels: Vec<Box<dyn Panel>>,
}

impl Panels {
    pub fn new() -> Self {
        Self { panels: vec![] }
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.panels.len()
    }

    pub fn push(&mut self, panel: Box<dyn Panel>) -> usize {
        for (i, p) in self.panels.iter_mut().enumerate() {
            if p.panel_type() == NULL_PANEL_TYPE_ID {
                *p = panel;
                return i;
            }
        }

        // add new if no empty slots
        self.panels.push(panel);
        self.panels.len() - 1
    }

    pub fn remove(&mut self, index: usize) {
        match self.panels.get_mut(index) {
            None => (),
            Some(panel) => *panel = PanelFactory::null(),
        }
    }

    pub fn get(&self, index: usize) -> Option<&Box<dyn Panel>> {
        self.panels.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut Box<dyn Panel>> {
        self.panels.get_mut(index)
    }
}

#[cfg(test)]
mod tests {
    use crate::panels::{PanelFactory, Panels, NULL_PANEL_TYPE_ID};

    #[test]
    fn add_panel() {
        let mut panels = Panels::new();
        let index = panels.push(PanelFactory::panel("Edit").unwrap());
        assert_eq!(index, 0);
    }

    #[test]
    fn remove_panel() {
        let mut panels = Panels::new();
        let index = panels.push(PanelFactory::panel("Edit").unwrap());
        panels.remove(index);

        assert_eq!(panels.panels[0].panel_type(), NULL_PANEL_TYPE_ID);
    }

    #[test]
    fn add_after_remove() {
        let mut panels = Panels::new();
        panels.push(PanelFactory::panel("Edit").unwrap());
        panels.push(PanelFactory::panel("Edit").unwrap());

        panels.remove(0);

        let index = panels.push(PanelFactory::panel("Edit").unwrap());

        assert_eq!(index, 0);
    }
}
