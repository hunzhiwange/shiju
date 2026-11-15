use super::*;
pub(super) fn view() -> Element<'static,Message> { domain::templates::view().map(map) }
