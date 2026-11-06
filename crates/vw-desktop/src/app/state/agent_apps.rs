//! Application catalog state is independent of the custom icon library.
use super::AgentAppsContext;
use crate::app::components::system_settings_apps::catalog::CatalogRow;

#[derive(Clone, Default)]
pub(crate) struct AppsCatalogState {
    pub lifecycle: crate::app::components::system_settings_apps::lifecycle::LifecycleState,
    pub host: String,
    pub token: String,
    pub connection_open: bool,
    pub loading: bool,
    pub error: Option<String>,
    pub rows: Vec<CatalogRow>,
    pub context: Option<AgentAppsContext>,
    pub generation: u64,
}
// Credentials must never enter App/Message debug output.
impl std::fmt::Debug for AppsCatalogState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppsCatalogState").field("loading", &self.loading).finish_non_exhaustive()
    }
}
impl AppsCatalogState {
    pub fn reset_if_context_changed(&mut self, context: &AgentAppsContext) -> bool {
        if self.context.as_ref() == Some(context) {
            return false;
        }
        self.generation = self.generation.wrapping_add(1);
        self.loading = false;
        self.rows.clear();
        self.token.clear();
        self.lifecycle.invalidate();
        self.connection_open = true;
        self.error = Some("连接上下文已更改，请重新输入应用管理令牌".into());
        true
    }

    pub fn begin(&mut self, context: AgentAppsContext) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.context = Some(context);
        self.loading = true;
        self.rows.clear();
        self.error = None;
        self.generation
    }
    pub fn finish(
        &mut self,
        context: &AgentAppsContext,
        generation: u64,
        result: Result<Vec<CatalogRow>, String>,
    ) -> bool {
        if self.context.as_ref() != Some(context) || self.generation != generation || !self.loading
        {
            return false;
        }
        self.loading = false;
        match result {
            Ok(rows) => self.rows = rows,
            Err(error) => self.error = Some(error),
        }
        true
    }
}
