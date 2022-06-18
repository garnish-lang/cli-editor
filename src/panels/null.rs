use crossterm::event::KeyEvent;
use tui::layout::Rect;
use tui::widgets::Block;
use crate::{EditorFrame, Panel};

pub struct NullPanel {
    title: String
}

impl NullPanel {
    pub fn new() -> Self {
        NullPanel {
            title: "".to_string()
        }
    }
}

impl Panel for NullPanel {
    fn get_active(&self) -> bool {
        false
    }
}