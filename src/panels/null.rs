use crate::Panel;

pub struct NullPanel {}

impl NullPanel {
    pub fn new() -> Self {
        NullPanel {}
    }
}

impl Panel for NullPanel {
    fn get_active(&self) -> bool {
        false
    }
}
