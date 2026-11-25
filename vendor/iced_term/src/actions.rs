#[derive(Debug, Clone, PartialEq, Default)]
pub enum Action {
    Shutdown,
    ChangeTitle(String),
    OpenPath(String),
    #[default]
    Ignore,
}
