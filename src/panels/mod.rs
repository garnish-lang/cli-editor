use crossterm::event::KeyEvent;
use tui::layout::Rect;
use tui::widgets::Block;

pub use edit::TextEditPanel;
pub use input::InputPanel;
pub use null::NullPanel;
pub use messages::MessagesPanel;

use crate::app::StateChangeRequest;
use crate::{AppState, EditorFrame};

mod edit;
mod input;
mod null;
mod messages;
mod factory;

pub trait Panel {
    fn type_id(&self) -> &str;
    fn init(&mut self, _state: &mut AppState) {}
    fn make_widget(&self, _state: &AppState, _frame: &mut EditorFrame, _rect: Rect, _is_active: bool, _block: Block) {}
    fn get_cursor(&self, _rect: &Rect) -> (u16, u16) {
        (0, 0)
    }
    fn get_title(&self) -> &str {
        ""
    }
    fn set_title(&mut self, _title: String) {}
    fn get_length(&self) -> u16 {
        0
    }
    fn get_id(&self) -> char {
        '\0'
    }
    fn set_id(&mut self, _id: char) {}
    fn receive_key(&mut self, _event: KeyEvent) -> (bool, Vec<StateChangeRequest>) {
        (false, vec![])
    }
    fn set_active(&mut self) {}
    fn get_active(&self) -> bool {
        true
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
