use crate::panels::{EDIT_PANEL_TYPE_ID, MESSAGE_PANEL_TYPE_ID, MessagesPanel, NULL_PANEL_TYPE_ID, NullPanel};
use crate::{InputPanel, TextEditPanel, TextPanel};

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

    pub fn panel(type_id: &str) -> Option<TextPanel> {
        match type_id {
            NULL_PANEL_TYPE_ID => Some(TextPanel::default()),
            EDIT_PANEL_TYPE_ID => Some(TextPanel::edit_panel()),
            MESSAGE_PANEL_TYPE_ID => Some(TextPanel::messages_panel()),
            _ => None,
        }
    }

    pub fn null() -> TextPanel {
        TextPanel::default()
    }

    pub fn input() -> TextPanel {
        TextPanel::input_panel()
    }

    pub fn messages() -> TextPanel {
        TextPanel::messages_panel()
    }

    pub fn edit() -> TextPanel {
        TextPanel::edit_panel()
    }
}

#[cfg(test)]
mod tests {
    use crate::panels::factory::PanelFactory;
    use crate::panels::{EDIT_PANEL_TYPE_ID, MESSAGE_PANEL_TYPE_ID, NULL_PANEL_TYPE_ID};

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
            PanelFactory::panel(NULL_PANEL_TYPE_ID)
                .unwrap()
                .panel_type(),
            NULL_PANEL_TYPE_ID
        );
    }

    #[test]
    fn create_edit_boxed() {
        assert_eq!(
            PanelFactory::panel(EDIT_PANEL_TYPE_ID)
                .unwrap()
                .panel_type(),
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
