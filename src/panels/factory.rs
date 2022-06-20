pub use crate::panels::edit::EDIT_PANEL_TYPE_ID;
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
            MESSAGE_PANEL_TYPE_ID,
        ]
    }

    pub fn panel(type_id: &str) -> Option<Box<dyn Panel>> {
        match type_id {
            NULL_PANEL_TYPE_ID => Some(Box::new(NullPanel::new())),
            EDIT_PANEL_TYPE_ID => Some(Box::new(TextEditPanel::new())),
            MESSAGE_PANEL_TYPE_ID => Some(Box::new(MessagesPanel::new())),
            _ => None,
        }
    }

    pub fn null() -> Box<dyn Panel> {
        Box::new(NullPanel::new())
    }

    pub fn input() -> Box<dyn Panel> {
        Box::new(InputPanel::new())
    }

    pub fn edit() -> Box<dyn Panel> {
        Box::new(TextEditPanel::new())
    }
}

#[cfg(test)]
mod tests {
    use crate::panels::edit::EDIT_PANEL_TYPE_ID;
    use crate::panels::factory::PanelFactory;
    use crate::panels::messages::MESSAGE_PANEL_TYPE_ID;
    use crate::panels::null::NULL_PANEL_TYPE_ID;

    #[test]
    fn get_available() {
        assert_eq!(
            PanelFactory::options(),
            vec![
                NULL_PANEL_TYPE_ID,
                EDIT_PANEL_TYPE_ID,
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
            PanelFactory::panel(NULL_PANEL_TYPE_ID).unwrap().panel_type(),
            NULL_PANEL_TYPE_ID
        );
    }

    #[test]
    fn create_edit_boxed() {
        assert_eq!(
            PanelFactory::panel(EDIT_PANEL_TYPE_ID).unwrap().panel_type(),
            EDIT_PANEL_TYPE_ID
        );
    }

    #[test]
    fn create_message_boxed() {
        assert_eq!(
            PanelFactory::panel(MESSAGE_PANEL_TYPE_ID)
                .unwrap()
                .panel_type(),
            MESSAGE_PANEL_TYPE_ID
        );
    }
}
