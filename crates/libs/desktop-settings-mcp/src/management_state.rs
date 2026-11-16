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
