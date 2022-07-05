use crossterm::event::KeyEvent;
use tui::layout::{Direction, Rect};
use tui::text::Span;

pub use factory::*;
pub use input::InputPanel;
pub use messages::MessagesPanel;
pub use text::{TextPanel};

use crate::app::StateChangeRequest;
use crate::{AppState, EditorFrame};

mod edit;
mod factory;
mod input;
mod messages;
mod text;
mod commands;

pub type PanelTypeID = &'static str;

pub const EDIT_PANEL_TYPE_ID: &str = "Edit";
pub const INPUT_PANEL_TYPE_ID: &str = "Input";
pub const COMMANDS_PANEL_TYPE_ID: &str = "Commands";
pub const MESSAGE_PANEL_TYPE_ID: &str = "Messages";
pub const NULL_PANEL_TYPE_ID: &str = "Null";

pub struct Panels {
    panels: Vec<TextPanel>,
}

impl Panels {
    pub fn new() -> Self {
        Self { panels: vec![] }
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.panels.len()
    }

    pub fn push(&mut self, panel: TextPanel) -> usize {
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
            Some(panel) => *panel = TextPanel::default(),
        }
    }

    pub fn get(&self, index: usize) -> Option<&TextPanel> {
       self.panels.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut TextPanel> {
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
