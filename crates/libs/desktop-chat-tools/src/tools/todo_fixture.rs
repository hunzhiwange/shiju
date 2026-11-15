use std::collections::HashSet;
#[derive(Default)]
pub struct TestApp {
 base: super::view_fixture::TestApp,
 pub chat_tool_expanded: HashSet<u64>,
 pub active_session_id: Option<String>,
 pub chat_todo_session_id: Option<String>,
 pub chat_todo_items: Vec<todo_types::Todo>,
}
impl TestApp {
 pub fn new() -> (Self, ()) { (Self::default(), ()) }
 pub fn input(&self) -> crate::card_input::CardInput<'_> {
  let mut card = self.base.files().card;
  card.expanded = &self.chat_tool_expanded;
  card
 }
 pub fn todos(&self) -> Option<(Vec<todo_types::Todo>, Option<String>)> {
  (self.active_session_id.is_some() && self.active_session_id == self.chat_todo_session_id).then(|| (self.chat_todo_items.clone(), None))
 }
}
