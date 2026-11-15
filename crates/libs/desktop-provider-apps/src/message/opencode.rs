use iced::widget::text_editor;
#[derive(Clone)]
pub enum Field {
    Declaration(text_editor::Action),
    V2(bool),
    Apply(bool),
}
