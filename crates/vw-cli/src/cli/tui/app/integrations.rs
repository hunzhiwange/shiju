//! 集成目录与操作。

use super::{
    IntegrationItem,
    IntegrationKind,
    Result,
    TuiAction,
    UiConfirmIntent,
    UiConfirmOverlay,
    UiOverlay,
    V3GatewayApp,
    V3SelectionItem,
    command_palette_items_with_integrations,
    delete_integration_blocking,
    load_integrations_blocking,
    push_runner_error,
    reduce_tui_state,
    set_integration_enabled_blocking,
};

impl V3GatewayApp {
    pub(super) fn open_integrations(&mut self, kind: IntegrationKind) {
        let mut items = self
            .state
            .integration_catalog
            .iter()
            .filter(|item| item.kind == kind)
            .map(|item| V3SelectionItem {
                id: item.stable_key(),
                label: item.title.clone(),
                description: Some(format!(
                    "{} · {} · 来源 {}",
                    if item.enabled { "enabled" } else { "disabled" },
                    item.detail,
                    item.source,
                )),
                search_value: Some(format!("{} {} {}", item.id, item.title, item.source)),
                disabled: true,
                disabled_reason: Some("只读目录；管理操作使用带稳定 ID 的斜杠命令".to_string()),
                ..V3SelectionItem::default()
            })
            .collect::<Vec<_>>();
        if items.is_empty() {
            items.push(V3SelectionItem {
                id: "empty".to_string(),
                label: "无可用条目".to_string(),
                description: self.state.integration_warnings.first().cloned(),
                disabled: true,
                ..V3SelectionItem::default()
            });
        }
        self.widget.show_selection_overlay(
            format!("integration_{}", kind.command_name()),
            format!("{} 目录", kind.command_name()),
            items,
        );
    }

    pub(super) fn replace_memory_items(&mut self, mut items: Vec<IntegrationItem>) {
        self.state.integration_catalog.retain(|item| item.kind != IntegrationKind::Memory);
        self.state.integration_catalog.append(&mut items);
    }

    pub(super) fn expand_integration_prompt(
        &self,
        kind: IntegrationKind,
        id: &str,
        arguments: &str,
    ) -> Result<String, String> {
        let item = self.require_integration(kind, id)?;
        if !item.enabled {
            return Err(format!("integration {} is disabled", item.stable_key()));
        }
        let prefix = item
            .prompt_prefix
            .as_deref()
            .ok_or_else(|| "integration does not expose prompt expansion".to_string())?;
        Ok(if arguments.trim().is_empty() {
            prefix.to_string()
        } else {
            format!("{prefix}\n\n{}", arguments.trim())
        })
    }

    pub(super) fn open_integration_toggle_confirmation(
        &mut self,
        kind: IntegrationKind,
        id: &str,
        enabled: bool,
    ) {
        let result = self.require_integration(kind, id).and_then(|item| {
            if !item.can_toggle {
                return Err("selected integration cannot be toggled".to_string());
            }
            Ok(UiConfirmOverlay {
                request_id: format!("integration-toggle-{}-{id}", kind.command_name()),
                title: "确认更改集成状态".to_string(),
                body: format!("{} {}", if enabled { "启用" } else { "禁用" }, item.stable_key()),
                confirm_label: "确认".to_string(),
                cancel_label: "取消".to_string(),
                destructive: false,
                intent: UiConfirmIntent::IntegrationToggle { kind, id: id.to_string(), enabled },
            })
        });
        self.show_integration_confirmation(result);
    }

    pub(crate) fn open_integration_delete_confirmation(&mut self, kind: IntegrationKind, id: &str) {
        let result = self.require_integration(kind, id).and_then(|item| {
            if !item.can_delete {
                return Err("selected integration cannot be deleted".to_string());
            }
            Ok(UiConfirmOverlay {
                request_id: format!("integration-delete-{}-{id}", kind.command_name()),
                title: "确认删除集成".to_string(),
                body: format!("删除 {}", item.stable_key()),
                confirm_label: "删除".to_string(),
                cancel_label: "取消".to_string(),
                destructive: true,
                intent: UiConfirmIntent::IntegrationDelete { kind, id: id.to_string() },
            })
        });
        self.show_integration_confirmation(result);
    }

    pub(super) fn show_integration_confirmation(&mut self, result: Result<UiConfirmOverlay, String>) {
        match result {
            Ok(overlay) => {
                reduce_tui_state(
                    &mut self.state,
                    TuiAction::OverlayPushed(UiOverlay::Confirm(overlay)),
                );
                self.show_next_interaction();
            }
            Err(error) => push_runner_error(self, error),
        }
    }

    pub(super) fn mutate_integration(
        &mut self,
        kind: IntegrationKind,
        id: &str,
        enabled: Option<bool>,
    ) -> Result<(), String> {
        let item = self.require_integration(kind, id)?.clone();
        match enabled {
            Some(enabled) => set_integration_enabled_blocking(&self.runtime, &item, enabled)?,
            None => delete_integration_blocking(&self.runtime, &item)?,
        }
        let loaded = load_integrations_blocking(&self.runtime);
        self.state.integration_catalog = loaded.items;
        self.state.integration_warnings = loaded.warnings;
        self.widget.set_command_palette_items(command_palette_items_with_integrations(
            &self.state.integration_catalog,
        ));
        Ok(())
    }

    fn require_integration(
        &self,
        kind: IntegrationKind,
        id: &str,
    ) -> Result<&cli_interaction_model::integration::IntegrationItem, String> {
        let id = id.trim();
        if id.is_empty() {
            return Err("integration operation requires a stable id".to_string());
        }
        self.state
            .integration_catalog
            .iter()
            .find(|item| item.kind == kind && item.id == id)
            .ok_or_else(|| {
                format!("integration {}:{id} is not in the Gateway catalog", kind.command_name())
            })
    }
}
