use super::SETTINGS_CONTROL_TEXT_SIZE;
use crate::single_line_label::SingleLineLabel;
use iced::Element;

pub struct Label(SingleLineLabel);

impl Label {
    pub fn new(value: String) -> Self {
        Self(SingleLineLabel::new(value, SETTINGS_CONTROL_TEXT_SIZE))
    }
}

impl<Message> From<Label> for Element<'_, Message> {
    fn from(label: Label) -> Self {
        label.0.into()
    }
}
