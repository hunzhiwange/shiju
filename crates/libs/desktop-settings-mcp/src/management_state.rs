#[derive(Debug, Clone)]
#[derive(Default)]
pub struct McpSettingsState {
    pub connection_form: Option<crate::management_form::Draft>,
    pub page: crate::state::State,
    pub extensions: crate::extensions::State,
}

// Preserve existing callers until the update slices migrate; state has one owner.
impl std::ops::Deref for McpSettingsState {
    type Target = crate::state::State;
    fn deref(&self) -> &Self::Target {
        &self.page
    }
}
impl std::ops::DerefMut for McpSettingsState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.page
    }
}

impl McpSettingsState {
    pub fn mutating(&self) -> bool {
        self.page.saving || self.connection_form.as_ref().is_some_and(|d| d.saving)
    }
    pub fn modal_open(&self) -> bool {
        self.connection_form.is_some()
    }
}
