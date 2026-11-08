use super::SETTINGS_CONTROL_TEXT_SIZE;
use crate::app::components::widgets::SingleLineLabel;
use iced::Element;

pub(crate) struct Label(SingleLineLabel);

impl Label {
    pub(crate) fn new(value: String) -> Self {
        Self(SingleLineLabel::new(value, SETTINGS_CONTROL_TEXT_SIZE))
    }
}

impl<Message> From<Label> for Element<'_, Message> {
    fn from(label: Label) -> Self {
        label.0.into()
    }
}
