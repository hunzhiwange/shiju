use crate::card_input::CardInput;
#[derive(Clone)]
pub struct FilesInput<'a> {
 pub card: CardInput<'a>,
 pub filter: &'a str,
 pub file_hovered: Option<&'a str>,
}
impl<'a> std::ops::Deref for FilesInput<'a> {
 type Target = CardInput<'a>;
 fn deref(&self) -> &Self::Target { &self.card }
}
