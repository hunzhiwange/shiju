use crate::{Scope,state::edit::{EditForm,ProviderEditState}};
#[derive(Default)]
pub struct ListState { scope: Option<Scope>, generation: u64, pub form: Option<EditForm> }
impl ListState {
 pub fn begin(&mut self, scope: Scope)->u64 { self.scope=Some(scope); self.generation=self.generation.wrapping_add(1); self.form=None; self.generation }
 pub fn borrowed(&mut self)->ProviderEditState<'_> { ProviderEditState { scope:self.scope.as_ref(),generation:self.generation,form:&mut self.form } }
}
