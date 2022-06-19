use crate::Panel;

pub struct NullPanel {}

pub const NULL_PANEL_TYPE_ID: &str = "Null";

impl NullPanel {
    pub fn new() -> Self {
        NullPanel {}
    }
}

impl Panel for NullPanel {
    fn type_id(&self) -> &str {
        NULL_PANEL_TYPE_ID
    }

    fn get_active(&self) -> bool {
        false
    }

    fn visible(&self) -> bool {
        false
    }
}
