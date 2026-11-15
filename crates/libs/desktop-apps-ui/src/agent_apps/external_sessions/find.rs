#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub enum Filter {
    #[default]
    All,
    Conversation,
    Changes,
}
#[derive(Clone, Default)]
pub struct State {
    pub query: String,
    pub selected: usize,
    pub filter: Filter,
}
#[derive(Clone, Debug)]
pub enum Event {
    Query(String),
    Move(bool),
    Filter(Filter),
}
