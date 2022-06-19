pub use crate::panels::edit::EDIT_PANEL_TYPE_ID;
pub use crate::panels::input::INPUT_PANEL_TYPE_ID;
pub use crate::panels::messages::MESSAGE_PANEL_TYPE_ID;
pub use crate::panels::null::NULL_PANEL_TYPE_ID;
use crate::panels::{MessagesPanel, NullPanel};
use crate::{InputPanel, Panel, TextEditPanel};

pub struct PanelFactory {}

#[allow(dead_code)]
impl PanelFactory {
    pub fn options() -> Vec<&'static str> {
        vec![
            NULL_PANEL_TYPE_ID,
            EDIT_PANEL_TYPE_ID,
            INPUT_PANEL_TYPE_ID,
            MESSAGE_PANEL_TYPE_ID,
        ]
    }

    pub fn panel(type_id: &str) -> Option<Box<dyn Panel>> {
        match type_id {
            NULL_PANEL_TYPE_ID => Some(Box::new(NullPanel::new())),
            EDIT_PANEL_TYPE_ID => Some(Box::new(TextEditPanel::new())),
            INPUT_PANEL_TYPE_ID => Some(Box::new(InputPanel::new())),
            MESSAGE_PANEL_TYPE_ID => Some(Box::new(MessagesPanel::new())),
            _ => None,
        }
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

    #[test]
    fn create_invalid() {
        assert!(PanelFactory::panel("Test").is_none());
    }

    #[test]
    fn create_null_boxed() {
        assert_eq!(
            PanelFactory::panel(NULL_PANEL_TYPE_ID).unwrap().type_id(),
            NULL_PANEL_TYPE_ID
        );
    }

    #[test]
    fn create_edit_boxed() {
        assert_eq!(
            PanelFactory::panel(EDIT_PANEL_TYPE_ID).unwrap().type_id(),
            EDIT_PANEL_TYPE_ID
        );
    }

    #[test]
    fn create_input_boxed() {
        assert_eq!(
            PanelFactory::panel(INPUT_PANEL_TYPE_ID).unwrap().type_id(),
            INPUT_PANEL_TYPE_ID
        );
    }

    #[test]
    fn create_message_boxed() {
        assert_eq!(
            PanelFactory::panel(MESSAGE_PANEL_TYPE_ID)
                .unwrap()
                .type_id(),
            MESSAGE_PANEL_TYPE_ID
        );
    }
}
