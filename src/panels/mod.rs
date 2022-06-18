use crossterm::event::KeyEvent;
use tui::layout::Rect;
use tui::widgets::Block;

pub use edit::TextEditPanel;
pub use null::NullPanel;
pub use input::InputPanel;

use crate::EditorFrame;

mod edit;
mod null;
mod input;

pub trait Panel {
    fn make_widget(&self, _frame: &mut EditorFrame, _rect: Rect, _is_active: bool, _block: Block) {}
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
    fn receive_key(&mut self, _event: KeyEvent) -> bool {
        false
    }
    fn set_active(&mut self) {}
    fn get_active(&self) -> bool {
        true
    }
}
