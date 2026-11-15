use iced::widget::text_editor;
#[derive(Clone)]
pub enum Field {
    Toml(text_editor::Action),
    AddModel,
    RemoveModel(usize),
    ModelId(usize, String),
    ModelName(usize, String),
    Apply(bool),
}
