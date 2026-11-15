/// Resolve a logout target within the authorized service and owner scope.
#[derive(Clone, Copy)]
pub enum LogoutSelection<'a> {
    Account(&'a str),
    Default,
    All,
}
