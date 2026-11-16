#[derive(Debug, Clone)]
pub struct McpSettingsState {
    pub tools: crate::management_tools::ToolsState,
    pub diagnostics: desktop_apps_ui::agent_apps_diagnostics::mcp_state::McpState,
    pub catalog: crate::management_list::CatalogState,
    pub resync: crate::management_resync::ResyncState,
    pub bindings: crate::management_bindings::BindingsState,
    pub import: Option<crate::management_import::ImportState>,
    pub connection_form: Option<crate::management_form::Draft>,
    pub page: crate::state::State,
}

impl Default for McpSettingsState {
    fn default() -> Self {
        Self {
            diagnostics: Default::default(),
            tools: Default::default(),
            catalog: Default::default(),
            bindings: Default::default(),
            resync: Default::default(),
            import: None,
            connection_form: None,
            page: Default::default(),
        }
    }
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
        self.page.saving
            || self.bindings.running
            || self.connection_form.as_ref().is_some_and(|d| d.saving)
    }
    pub fn modal_open(&self) -> bool {
        self.import.is_some() || self.connection_form.is_some()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlay {
    Tools,
    Resync,
    Import,
    Form,
    Page,
}
impl McpSettingsState {
    pub fn overlay(
        &self,
        current: Option<&crate::management_context::Context>,
        diagnostics: Option<&crate::diagnostic_context::Context>,
    ) -> Overlay {
        if crate::management_tools::visible(&self.tools, diagnostics).is_some() {
            return Overlay::Tools;
        }
        if crate::management_resync::visible(&self.resync, current).is_some_and(|s| s.open) {
            return Overlay::Resync;
        }
        if crate::management_import::visible(self.import.as_ref(), current).is_some() {
            return Overlay::Import;
        }
        if self.connection_form.is_some() {
            return Overlay::Form;
        }
        Overlay::Page
    }
}
