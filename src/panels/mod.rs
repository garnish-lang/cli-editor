use crossterm::event::KeyEvent;
use tui::layout::{Direction, Rect};
use tui::text::Span;

pub use edit::TextEditPanel;
pub use factory::*;
pub use input::InputPanel;
pub use messages::MessagesPanel;
pub use null::NullPanel;
pub use text::TextPanel;

use crate::app::StateChangeRequest;
use crate::{AppState, EditorFrame};

mod edit;
mod factory;
mod input;
mod messages;
mod null;
mod text;

// pub struct RenderDetails<'a> {
//     pub title: Vec<Span<'a>>,
//     pub cursor: (u16, u16),
// }
//
// impl<'a> RenderDetails<'a> {
//     pub fn new(title: Vec<Span<'a>>, cursor: (u16, u16)) -> Self {
//         Self { title, cursor }
//     }
// }
//
// pub trait Panel {
//     fn panel_type(&self) -> &str;
//     fn init(&mut self, _state: &mut AppState) {}
//     fn make_widget(
//         &self,
//         _state: &AppState,
//         _frame: &mut EditorFrame,
//         _rect: Rect,
//         _is_active: bool,
//     ) -> RenderDetails {
//         RenderDetails {
//             title: vec![],
//             cursor: (0, 0),
//         }
//     }
//     fn get_length(
//         &self,
//         _fixed_length: u16,
//         _flex_length: u16,
//         _direction: Direction,
//         _state: &AppState,
//     ) -> u16 {
//         0
//     }
//     fn receive_key(
//         &mut self,
//         _event: KeyEvent,
//         _state: &mut AppState,
//     ) -> (bool, Vec<StateChangeRequest>) {
//         (false, vec![])
//     }
//     fn receive_input(&mut self, _input: String) -> Vec<StateChangeRequest> {
//         vec![]
//     }
//     fn show(&mut self) {}
//     fn hide(&mut self) {}
//     fn visible(&self) -> bool {
//         true
//     }
// }

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
