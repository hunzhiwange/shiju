use iced::widget::text_editor;
#[derive(Clone)]
pub enum Field {
    Toml(text_editor::Action),
    Apply(bool),
}
