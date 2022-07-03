pub struct NullPanel {}

impl NullPanel {
    pub fn new() -> Self {
        NullPanel {}
    }
}

// impl Panel for NullPanel {
//     fn panel_type(&self) -> &str {
//         NULL_PANEL_TYPE_ID
//     }
//
//     fn visible(&self) -> bool {
//         false
//     }
// }
