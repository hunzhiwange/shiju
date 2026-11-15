//! 运行器面板请求。

use super::{
    ActiveHookMutation,
    ActiveRunner,
    Arc,
    HookCatalogItem,
    HostMessage,
    IntegrationKind,
    MemoryMutation,
    Result,
    UiConfirmIntent,
    UiConfirmOverlay,
    V3DisplayEvent,
    V3GatewayApp,
    spawn_cron_detail,
    spawn_doctor,
    spawn_file_search,
    spawn_hook_audit,
    spawn_hook_catalog,
    spawn_hook_mutation,
    spawn_memory_mutation,
    spawn_memory_search,
    spawn_session_diff,
    spawn_task_detail,
    spawn_workspace_file_read,
    spawn_workspace_files,
};

impl ActiveRunner {
    pub(super) fn ensure_session_mutation_idle(&self, app: &V3GatewayApp) -> Result<(), String> {
        if self.directory_switch_source.is_some() {
            return Err("cannot mutate a session while a directory switch is active".to_string());
        }
        if self.active_stream.is_some() {
            return Err("cannot mutate a session while a gateway stream is active".to_string());
        }
        if app.widget().queued_input_count() > 0 {
            return Err("cannot mutate a session while queued prompts are pending".to_string());
        }
        Ok(())
    }

    pub(super) fn launch_editor(&mut self, draft: String) {
        if self.editor_in_flight {
            return;
        }
        self.editor_in_flight = true;
        let editor = Arc::clone(&self.editor);
        let sender = self.host_sender.clone();
        std::thread::spawn(move || {
            let result = editor(draft.as_str());
            let _ = sender.send(HostMessage::EditorFinished(result));
        });
    }

    pub(super) fn start_file_search(&mut self, app: &V3GatewayApp, query: Option<String>) {
        self.file_search_generation = self.file_search_generation.wrapping_add(1);
        self.file_search_query.clone_from(&query);
        let Some(query) = query else {
            return;
        };
        spawn_file_search(
            app.runtime().clone(),
            self.file_search_generation,
            query,
            self.host_sender.clone(),
        );
    }

    pub(super) fn start_session_diff(&mut self, app: &V3GatewayApp) {
        self.session_diff_generation = self.session_diff_generation.wrapping_add(1);
        spawn_session_diff(
            app.runtime().clone(),
            self.session_diff_generation,
            self.host_sender.clone(),
        );
    }

    pub(super) fn start_doctor(&mut self, app: &mut V3GatewayApp) {
        self.doctor_generation = self.doctor_generation.wrapping_add(1);
        app.widget_mut().push_history(V3DisplayEvent::System {
            text: "正在运行 Gateway 诊断…".to_string(),
        });
        spawn_doctor(app.runtime().clone(), self.doctor_generation, self.host_sender.clone());
    }

    pub(super) fn start_cron_detail(&mut self, app: &V3GatewayApp, job_id: String) {
        self.cron_generation = self.cron_generation.wrapping_add(1);
        spawn_cron_detail(
            app.runtime().clone(),
            app.state().orchestration.cron_jobs.clone(),
            self.cron_generation,
            job_id,
            self.host_sender.clone(),
        );
    }

    pub(super) fn start_task_detail(&mut self, app: &V3GatewayApp, task_id: String) {
        self.task_detail_generation = self.task_detail_generation.wrapping_add(1);
        spawn_task_detail(
            app.runtime().clone(),
            app.state().orchestration.clone(),
            self.task_detail_generation,
            task_id,
            self.host_sender.clone(),
        );
    }

    pub(super) fn start_workspace_files(&mut self, app: &V3GatewayApp, path: String) {
        self.workspace_files_generation = self.workspace_files_generation.wrapping_add(1);
        spawn_workspace_files(
            app.runtime().clone(),
            self.workspace_files_generation,
            path,
            self.host_sender.clone(),
        );
    }

    pub(super) fn start_workspace_file_read(&mut self, app: &V3GatewayApp, path: String) {
        self.workspace_files_generation = self.workspace_files_generation.wrapping_add(1);
        spawn_workspace_file_read(
            app.runtime().clone(),
            self.workspace_files_generation,
            path,
            self.host_sender.clone(),
        );
    }

    pub(super) fn start_hook_catalog(&mut self, app: &V3GatewayApp) {
        self.hooks_generation = self.hooks_generation.wrapping_add(1);
        spawn_hook_catalog(app.runtime().clone(), self.hooks_generation, self.host_sender.clone());
    }

    pub(super) fn start_hook_audit(&mut self, app: &V3GatewayApp) {
        self.hooks_generation = self.hooks_generation.wrapping_add(1);
        spawn_hook_audit(app.runtime().clone(), self.hooks_generation, self.host_sender.clone());
    }

    pub(super) fn start_memory_search(&mut self, app: &V3GatewayApp, query: Option<String>) {
        self.memory_generation = self.memory_generation.wrapping_add(1);
        spawn_memory_search(
            app.runtime().clone(),
            self.memory_generation,
            query,
            self.host_sender.clone(),
        );
    }

    pub(super) fn open_memory_store_confirmation(&self, app: &mut V3GatewayApp, key: &str, content: String) {
        let key = key.trim();
        let result = if self.memory_mutation_generation.is_some() {
            Err("a memory mutation is already in progress".to_string())
        } else {
            Ok(UiConfirmOverlay {
                request_id: format!("memory-store-{key}"),
                title: "确认写入 core 记忆".to_string(),
                body: format!("写入 core 记忆 Key: {key}\n服务端同名记忆可能被覆盖。"),
                confirm_label: "写入".to_string(),
                cancel_label: "取消".to_string(),
                destructive: true,
                intent: UiConfirmIntent::MemoryStore { key: key.to_string(), content },
            })
        };
        app.show_integration_confirmation(result);
    }

    pub(super) fn open_memory_delete_confirmation(&self, app: &mut V3GatewayApp, key: &str) {
        let result = if self.memory_mutation_generation.is_some() {
            Err("a memory mutation is already in progress".to_string())
        } else {
            self.require_memory_key(app, key).map(|()| UiConfirmOverlay {
                request_id: format!("memory-delete-{}", key.trim()),
                title: "确认删除记忆".to_string(),
                body: format!("删除当前目录记忆 Key: {}", key.trim()),
                confirm_label: "删除".to_string(),
                cancel_label: "保留记忆".to_string(),
                destructive: true,
                intent: UiConfirmIntent::MemoryDelete { key: key.trim().to_string() },
            })
        };
        app.show_integration_confirmation(result);
    }

    pub(super) fn require_memory_key(&self, app: &V3GatewayApp, key: &str) -> Result<(), String> {
        if self.memory_snapshot_directory.as_deref() != Some(app.runtime().directory()) {
            return Err(
                "memory deletion requires a query snapshot from the current directory".to_string()
            );
        }
        let key = key.trim();
        app.state()
            .integration_catalog
            .iter()
            .any(|item| item.kind == IntegrationKind::Memory && item.id == key)
            .then_some(())
            .ok_or_else(|| {
                "memory deletion requires an exact key from the current memory snapshot".to_string()
            })
    }

    pub(super) fn start_memory_mutation(
        &mut self,
        app: &V3GatewayApp,
        mutation: MemoryMutation,
    ) -> Result<(), String> {
        if self.directory_switch_source.is_some() {
            return Err("cannot mutate memory while a directory switch is active".to_string());
        }
        if self.memory_mutation_generation.is_some() {
            return Err("a memory mutation is already in progress".to_string());
        }
        if matches!(mutation, MemoryMutation::Delete { .. }) {
            self.require_memory_key(app, mutation.key())?;
        }
        let query = (self.memory_snapshot_directory.as_deref() == Some(app.runtime().directory()))
            .then(|| self.memory_snapshot_query.clone())
            .flatten();
        self.memory_generation = self.memory_generation.wrapping_add(1);
        self.memory_mutation_generation = Some(self.memory_generation);
        spawn_memory_mutation(
            app.runtime().clone(),
            self.memory_generation,
            mutation,
            query,
            self.host_sender.clone(),
        );
        Ok(())
    }

    pub(super) fn open_hook_mutation_confirmation(
        &self,
        app: &mut V3GatewayApp,
        plugin_id: &str,
        hook_hash: &str,
        trusted: bool,
    ) {
        let result = self.current_hook(app, plugin_id, hook_hash).and_then(|hook| {
            if hook.trusted == trusted {
                return Err(format!(
                    "Hook {plugin_id} {hook_hash} is already {}",
                    if trusted { "trusted" } else { "revoked" }
                ));
            }
            let action = if trusted { "信任" } else { "撤销信任" };
            Ok(UiConfirmOverlay {
                request_id: format!(
                    "hook-{}-{plugin_id}-{hook_hash}",
                    if trusted { "trust" } else { "revoke" }
                ),
                title: format!("确认{action} Hook"),
                body: format!(
                    "{action}当前目录声明\nPlugin: {}\nEvent: {}\nMode: {}\nHash: {}",
                    hook.plugin_id, hook.event, hook.mode, hook.hook_hash
                ),
                confirm_label: action.to_string(),
                cancel_label: "取消".to_string(),
                destructive: true,
                intent: UiConfirmIntent::HookMutation {
                    plugin_id: hook.plugin_id.clone(),
                    hook_hash: hook.hook_hash.clone(),
                    trusted,
                },
            })
        });
        app.show_integration_confirmation(result);
    }

    pub(super) fn start_hook_mutation(
        &mut self,
        app: &V3GatewayApp,
        plugin_id: &str,
        hook_hash: &str,
        trusted: bool,
    ) -> Result<(), String> {
        if self.hook_mutation.is_some() {
            return Err("a Hook mutation is already in progress".to_string());
        }
        let hook = self.current_hook(app, plugin_id, hook_hash)?;
        if hook.trusted == trusted {
            return Err("Hook catalog changed before confirmation was applied".to_string());
        }
        self.hooks_generation = self.hooks_generation.wrapping_add(1);
        self.hook_mutation = Some(ActiveHookMutation {
            plugin_id: plugin_id.to_string(),
            hook_hash: hook_hash.to_string(),
            trusted,
        });
        spawn_hook_mutation(
            app.runtime().clone(),
            self.hooks_generation,
            plugin_id.to_string(),
            hook_hash.to_string(),
            trusted,
            self.host_sender.clone(),
        );
        Ok(())
    }

    fn current_hook(
        &self,
        app: &V3GatewayApp,
        plugin_id: &str,
        hook_hash: &str,
    ) -> Result<HookCatalogItem, String> {
        if self.hook_catalog_directory.as_deref() != Some(app.runtime().directory()) {
            return Err("Hook mutation requires the current directory catalog snapshot".to_string());
        }
        self.hook_catalog
            .as_ref()
            .and_then(|catalog| catalog.find(plugin_id, hook_hash))
            .cloned()
            .ok_or_else(|| {
                "Hook mutation requires an exact plugin ID/hash from the current catalog"
                    .to_string()
            })
    }
}
