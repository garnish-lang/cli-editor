use crate::panels::edit::EDIT_PANEL_TYPE_ID;
use crate::panels::input::INPUT_PANEL_TYPE_ID;
use crate::panels::messages::MESSAGE_PANEL_TYPE_ID;
use crate::panels::null::NULL_PANEL_TYPE_ID;

pub struct PanelFactory {}

impl PanelFactory {
    pub fn options() -> Vec<&'static str> {
        vec![
            NULL_PANEL_TYPE_ID,
            EDIT_PANEL_TYPE_ID,
            INPUT_PANEL_TYPE_ID,
            MESSAGE_PANEL_TYPE_ID,
        ]
    }
}

#[cfg(test)]
mod tests {
    use crate::panels::edit::EDIT_PANEL_TYPE_ID;
    use crate::panels::factory::PanelFactory;
    use crate::panels::input::INPUT_PANEL_TYPE_ID;
    use crate::panels::messages::MESSAGE_PANEL_TYPE_ID;
    use crate::panels::null::NULL_PANEL_TYPE_ID;

    #[test]
    fn get_available() {
        assert_eq!(
            PanelFactory::options(),
            vec![
                NULL_PANEL_TYPE_ID,
                EDIT_PANEL_TYPE_ID,
                INPUT_PANEL_TYPE_ID,
                MESSAGE_PANEL_TYPE_ID,
            ]
        )
    }
}
