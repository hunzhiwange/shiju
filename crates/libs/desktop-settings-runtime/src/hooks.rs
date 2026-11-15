use crate::state::HooksSettingsState;
use crate::messages::HooksMessage;
pub enum Effect {
 None, CatalogRefresh, AuditRefresh,
 Trust { plugin_id: String, hook_hash: String }, Revoke { plugin_id: String, hook_hash: String },
 Save { enabled: bool, command_logger: bool, plugins_enabled: bool },
 Error { source: &'static str, error: String }, MutationSucceeded { source: &'static str },
}
pub fn update(state: &mut HooksSettingsState, message: HooksMessage) -> Effect {
    match message {
        HooksMessage::EnabledToggled(value) => {
            state.enabled = value;
        }
        HooksMessage::CommandLoggerToggled(value) => {
            state.command_logger = value;
        }
        HooksMessage::PluginHooksToggled(value) => {
            state.plugins_enabled = value;
        }
        HooksMessage::CatalogRefresh => {
            state.catalog_loading = true;
            state.catalog_error = None;
            return Effect::CatalogRefresh;
        }
        HooksMessage::CatalogLoaded(Ok(catalog)) => {
            state.catalog = catalog;
            state.catalog_loading = false;
            state.catalog_error = None;
            return Effect::None;
        }
        HooksMessage::CatalogLoaded(Err(error)) => {
            state.catalog_loading = false;
            state.catalog_error = Some(error.clone());
            return Effect::Error { source: "读取插件 hook 清单失败：%s", error };
        }
        HooksMessage::AuditRefresh => {
            state.audit_loading = true;
            state.audit_error = None;
            return Effect::AuditRefresh;
        }
        HooksMessage::AuditLoaded(Ok(audit)) => {
            state.audit = audit;
            state.audit_loading = false;
            state.audit_error = None;
            return Effect::None;
        }
        HooksMessage::AuditLoaded(Err(error)) => {
            state.audit_loading = false;
            state.audit_error = Some(error.clone());
            return Effect::Error { source: "读取 hook 审计失败：%s", error };
        }
        HooksMessage::TrustRequested { plugin_id, hook_hash } => {
            state.mutation_loading = true;
            state.mutation_error = None;
            return Effect::Trust { plugin_id, hook_hash };
        }
        HooksMessage::RevokeRequested { plugin_id, hook_hash } => {
            state.mutation_loading = true;
            state.mutation_error = None;
            return Effect::Revoke { plugin_id, hook_hash };
        }
        HooksMessage::MutationCompleted(Ok(result)) => {
            state.mutation_loading = false;
            state.mutation_error = None;
            if let Some(entry) = state.catalog.iter_mut().find(|entry| {
                entry.plugin_id == result.plugin_id && entry.hook_hash == result.hook_hash
            }) {
                entry.trusted = result.trusted;
            }
            let source = if result.trusted {
                "已信任插件 hook"
            } else {
                "已撤销信任插件 hook"
            };
            return Effect::MutationSucceeded { source };
        }
        HooksMessage::MutationCompleted(Err(error)) => {
            state.mutation_loading = false;
            state.mutation_error = Some(error.clone());
            return Effect::Error { source: "更新插件 hook 信任失败：%s", error };
        }
    }

    state.save_error = None;
    Effect::Save { enabled: state.enabled, command_logger: state.command_logger, plugins_enabled: state.plugins_enabled }
}
