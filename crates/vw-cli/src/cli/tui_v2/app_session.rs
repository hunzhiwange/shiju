//! TUI session restore/save 副作用。

use super::{
    ChatSessionMeta, CliSetup, Config, SessionRestoreOutcome, TuiAction, TuiApp, UiMessage,
    UiSystemMessage, UiSystemMessageLevel, bootstrap_message_base, bootstrap_system_message,
    AcpBackendOption, default_session_title, delegate_agent_options, empty_session_snapshot,
    load_model_catalog_blocking, normalize_optional_str_ref, provider_name_from_model,
    reduce_tui_state,
    select_restore_session_id, snapshot_from_preview,
};
use crate::cli::tui_v2::completion::refresh_file_completion_index;
use crate::cli::tui_v2::model::integration::{IntegrationKind, IntegrationPanel};
use crate::cli::tui_v2::runtime::integrations::refresh_integrations;

impl TuiApp {
    pub(super) fn refresh_completion_file_index_blocking(&mut self) {
        let workspace_root = self.state.project.workspace_root.clone();
        let mut roots = Vec::new();
        let mut has_unreadable_root = false;
        for candidate in workspace_root
            .iter()
            .chain(self.state.completion_catalog.added_directories.iter())
        {
            match candidate.canonicalize() {
                Ok(root) if root.is_dir() => roots.push(root),
                Ok(_) | Err(_) => has_unreadable_root = true,
            }
        }
        roots.sort();
        roots.dedup();
        let index = &self.state.completion_catalog.file_index;
        if index.revision > 0 && !has_unreadable_root && index.roots == roots {
            return;
        }
        refresh_file_completion_index(
            &mut self.state.completion_catalog,
            workspace_root.as_deref(),
        );
    }

    pub(super) fn bootstrap_session_state_blocking(&mut self, config: &Config, setup: &CliSetup) {
        reduce_tui_state(
            &mut self.state,
            TuiAction::ModelCatalogReplaced(load_model_catalog_blocking()),
        );
        self.state.session_capabilities.models =
            self.state.model_catalog.iter().map(|entry| entry.qualified_id()).collect();
        self.state.session_capabilities.efforts = vec![
            crate::cli::tui_v2::model::ReasoningEffort::Minimal,
            crate::cli::tui_v2::model::ReasoningEffort::Low,
            crate::cli::tui_v2::model::ReasoningEffort::Medium,
            crate::cli::tui_v2::model::ReasoningEffort::High,
            crate::cli::tui_v2::model::ReasoningEffort::XHigh,
        ];
        self.state.session_capabilities.permission_modes = vec![
            crate::cli::tui_v2::model::PermissionMode::Default,
            crate::cli::tui_v2::model::PermissionMode::Plan,
        ];
        self.state.session_capabilities.delegate_agents = delegate_agent_options(
            config,
            self.state.session_options.delegate_agent.as_deref(),
        );
        self.state.session_capabilities.acp_backends =
            match self.runtime.enabled_acp_backend_keys_blocking() {
                Ok(keys) => std::iter::once(AcpBackendOption::default_backend())
                    .chain(keys.into_iter().map(|key| AcpBackendOption {
                        label: key.clone(),
                        key: Some(key),
                    }))
                    .collect(),
                Err(error) => {
                    tracing::warn!(
                        error = %error,
                        "failed to load enabled ACP backends for TUI settings"
                    );
                    vec![AcpBackendOption::default_backend()]
                }
            };

        let restored = self.restore_session_snapshot_blocking();
        if !restored {
            self.ensure_session_binding_blocking();
        }
        self.sync_session_metadata_blocking();

        if self.state.session.title.trim().is_empty() {
            let title = self
                .runtime
                .title()
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| default_session_title(self.runtime.directory()));
            reduce_tui_state(&mut self.state, TuiAction::SessionTitleSet(title));
        }

        if self.state.session.scope.is_none() {
            let scope = self.resolve_session_scope_blocking();
            reduce_tui_state(&mut self.state, TuiAction::SessionScopeSet(scope));
        }

        let active_model = normalize_optional_str_ref(self.state.status.model_name.as_deref())
            .map(ToOwned::to_owned)
            .or_else(|| {
                normalize_optional_str_ref(Some(setup.model_name.as_str())).map(ToOwned::to_owned)
            });
        let active_provider =
            normalize_optional_str_ref(self.state.status.provider_name.as_deref())
                .map(ToOwned::to_owned)
                .or_else(|| active_model.as_deref().and_then(provider_name_from_model))
                .or_else(|| {
                    normalize_optional_str_ref(Some(setup.provider_name.as_str()))
                        .map(ToOwned::to_owned)
                });

        reduce_tui_state(&mut self.state, TuiAction::StatusProviderSet(active_provider));
        reduce_tui_state(&mut self.state, TuiAction::StatusModelSet(active_model));

        if self.state.messages.is_empty() {
            reduce_tui_state(
                &mut self.state,
                TuiAction::MessagePushed(UiMessage::System(UiSystemMessage {
                    base: bootstrap_message_base(),
                    text: bootstrap_system_message(self.run_mode),
                    level: UiSystemMessageLevel::Info,
                })),
            );
        }

        let mut skills = IntegrationPanel::new(IntegrationKind::Skill);
        refresh_integrations(&mut skills, &mut self.runtime);
        self.update_integration_commands(&skills);
    }

    fn restore_session_snapshot_blocking(&mut self) -> bool {
        let previews = match self.runtime.session_ui_previews_blocking() {
            Ok(previews) => previews,
            Err(err) => {
                reduce_tui_state(
                    &mut self.state,
                    TuiAction::StatusErrorSet(Some(format!("session preview sync failed: {err}"))),
                );
                Vec::new()
            }
        };

        let Some(session_id) = select_restore_session_id(self.runtime.session_id(), &previews)
        else {
            return false;
        };

        matches!(
            self.restore_session_snapshot_by_id_blocking(&previews, session_id.as_str(), true),
            SessionRestoreOutcome::Snapshot
        )
    }

    fn restore_session_snapshot_by_id_blocking(
        &mut self,
        previews: &[ChatSessionMeta],
        session_id: &str,
        allow_empty_binding: bool,
    ) -> SessionRestoreOutcome {
        let scope = self.resolve_session_scope_blocking();
        let path = self.runtime.session_path_blocking(Some(session_id)).ok().flatten();

        let snapshot = match self.runtime.session_ui_load_any_blocking(Some(session_id)) {
            Ok(snapshot) => snapshot,
            Err(err) => {
                reduce_tui_state(
                    &mut self.state,
                    TuiAction::StatusErrorSet(Some(format!("session restore failed: {err}"))),
                );
                None
            }
        };

        let Some(snapshot) = snapshot else {
            let preview = previews.iter().find(|preview| preview.id == session_id);
            let Some(snapshot) = preview
                .map(snapshot_from_preview)
                .or_else(|| allow_empty_binding.then(|| empty_session_snapshot(session_id)))
            else {
                return SessionRestoreOutcome::Missing;
            };

            reduce_tui_state(
                &mut self.state,
                TuiAction::ReplaceFromSnapshot { snapshot, scope, path },
            );
            return SessionRestoreOutcome::MetadataOnly;
        };

        reduce_tui_state(&mut self.state, TuiAction::ReplaceFromSnapshot { snapshot, scope, path });
        SessionRestoreOutcome::Snapshot
    }

    pub(super) fn restore_session_from_command(&mut self, session_id: Option<&str>) {
        if self.state.session.session_id.is_some() {
            self.persist_session_snapshot_blocking();
        }

        let previews = match self.runtime.session_ui_previews_blocking() {
            Ok(previews) => previews,
            Err(err) => {
                self.push_overlay_error("恢复失败", format!("会话预览同步失败: {err}"));
                return;
            }
        };

        let Some(target_session_id) = select_restore_session_id(session_id, &previews) else {
            self.push_overlay_error("恢复失败", "当前没有可供 TUI 恢复的会话快照。");
            return;
        };

        match self.restore_session_snapshot_by_id_blocking(
            &previews,
            target_session_id.as_str(),
            false,
        ) {
            SessionRestoreOutcome::Snapshot => {
                self.finish_session_restore();
                self.push_app_system_message(
                    format!("已恢复会话 {}", target_session_id),
                    UiSystemMessageLevel::Success,
                );
            }
            SessionRestoreOutcome::MetadataOnly => {
                self.finish_session_restore();
                self.push_app_system_message(
                    format!("会话 {} 还没有持久化快照；本次仅恢复了会话绑定。", target_session_id),
                    UiSystemMessageLevel::Warning,
                );
            }
            SessionRestoreOutcome::Missing => {
                self.push_overlay_error(
                    "恢复失败",
                    format!("没有找到 {} 对应的会话快照或预览。", target_session_id),
                );
            }
        }
    }

    pub(super) fn finish_session_restore(&mut self) {
        self.sync_session_metadata_blocking();
        if normalize_optional_str_ref(self.state.status.provider_name.as_deref()).is_none()
            && let Some(provider_name) =
                self.state.status.model_name.as_deref().and_then(provider_name_from_model)
        {
            reduce_tui_state(&mut self.state, TuiAction::StatusProviderSet(Some(provider_name)));
        }
        self.refresh_task_state_blocking();
        self.refresh_project_context_blocking();
        reduce_tui_state(&mut self.state, TuiAction::StatusErrorSet(None));
        self.state.refresh_session_preview();
    }

    pub(super) fn sync_runtime_session_seed_from_state(&mut self) {
        let title =
            (!self.state.session.title.trim().is_empty()).then(|| self.state.session.title.clone());
        self.runtime.bind_session_seed(
            self.state.session.session_id.clone(),
            self.state.session.scope.clone(),
            title,
        );
    }

    fn ensure_session_binding_blocking(&mut self) {
        if self.state.session.session_id.is_some() {
            self.sync_runtime_session_seed_from_state();
            return;
        }

        if let Some(session_id) = self.runtime.session_id() {
            self.state.session.session_id = Some(session_id.to_string());
            self.state.refresh_session_preview();
            self.sync_runtime_session_seed_from_state();
            return;
        }

        let title = self
            .runtime
            .title()
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| default_session_title(self.runtime.directory()));
        match self.runtime.session_create_blocking(Some(title.as_str())) {
            Ok(created) => {
                self.state.session.session_id = Some(created.id);
                self.state.session.title = created.title;
                self.state.refresh_session_preview();
                self.sync_runtime_session_seed_from_state();
            }
            Err(err) => {
                reduce_tui_state(
                    &mut self.state,
                    TuiAction::StatusErrorSet(Some(format!("session create failed: {err}"))),
                );
            }
        }
    }

    fn resolve_session_scope_blocking(&mut self) -> Option<String> {
        if let Some(scope) = self.runtime.scope().map(ToOwned::to_owned) {
            return Some(scope);
        }

        match self.runtime.session_scope_get_blocking() {
            Ok(scope) => scope,
            Err(err) => {
                reduce_tui_state(
                    &mut self.state,
                    TuiAction::StatusErrorSet(Some(format!("session scope sync failed: {err}"))),
                );
                None
            }
        }
    }

    pub(super) fn sync_session_metadata_blocking(&mut self) {
        let scope = self.resolve_session_scope_blocking();
        reduce_tui_state(&mut self.state, TuiAction::SessionScopeSet(scope));

        let Some(session_id) = self.state.session.session_id.clone() else {
            return;
        };

        if let Ok(Some(meta)) =
            self.runtime.session_preview_meta_blocking(Some(session_id.as_str()))
        {
            if !meta.title.trim().is_empty() {
                reduce_tui_state(&mut self.state, TuiAction::SessionTitleSet(meta.title.clone()));
            }
            reduce_tui_state(&mut self.state, TuiAction::SessionUpdatedMsSet(meta.updated_ms));
            reduce_tui_state(&mut self.state, TuiAction::SessionPreviewSet(Some(meta.into())));
        }

        match self.runtime.session_path_blocking(Some(session_id.as_str())) {
            Ok(Some(path)) => {
                reduce_tui_state(&mut self.state, TuiAction::SessionPathSet(Some(path)));
            }
            Ok(None) => {
                reduce_tui_state(&mut self.state, TuiAction::SessionPathSet(None));
            }
            Err(_) => {}
        }

        self.sync_runtime_session_seed_from_state();
    }

    pub(super) fn persist_session_snapshot_blocking(&mut self) {
        self.ensure_session_binding_blocking();

        let Some(session_id) = self.state.session.session_id.clone() else {
            return;
        };

        let mut snapshot = self.state.to_chat_session();
        if snapshot.id.trim().is_empty() {
            snapshot.id = session_id;
        }
        if snapshot.title.trim().is_empty() {
            snapshot.title = self.state.session.title.clone();
        }

        match self.runtime.session_ui_save_blocking(&snapshot) {
            Ok(()) => {
                self.sync_session_metadata_blocking();
            }
            Err(err) => {
                reduce_tui_state(
                    &mut self.state,
                    TuiAction::StatusErrorSet(Some(format!("session save failed: {err}"))),
                );
            }
        }
    }
}
