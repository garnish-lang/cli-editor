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
    fn type_id(&self) -> &str;
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
    fn get_cursor(&self, _rect: &Rect) -> (u16, u16) {
        (0, 0)
    }
    fn make_title(&self, _state: &AppState) -> Vec<Span> {
        vec![]
    }
    fn get_length(&self) -> u16 {
        0
    }
    fn receive_key(&mut self, _event: KeyEvent) -> (bool, Vec<StateChangeRequest>) {
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
