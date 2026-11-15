use iced::widget::text_editor;
#[derive(Clone)]
pub enum Field {
    Declaration(text_editor::Action),
    UserAgent(bool),
    Apply(bool),
}
