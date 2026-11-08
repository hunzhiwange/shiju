//! 合并编辑器设置补丁，并在关停前排空已提交的保存请求。

use crate::app::config::{gateway_client_endpoint, normalize_editor_metrics};
use crate::app::{App, Message};
use iced::Task;
use serde::Serialize;
use std::collections::VecDeque;
use vw_gateway_client::{GatewayClient, GatewayEndpoint};

use super::EditorMessage;

#[derive(Serialize)]
struct EditorMetrics {
    editor_font_size: f32,
    editor_line_height: f32,
    editor_auto_line_height: bool,
}

#[derive(Default, Serialize)]
pub(super) struct EditorSettingsPatch {
    #[serde(flatten)]
    metrics: Option<EditorMetrics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    editor_theme: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    editor_follow_system_theme: Option<bool>,
}

impl EditorSettingsPatch {
    pub(super) fn metrics(app: &App) -> Self {
        let (font_size, line_height) = normalize_editor_metrics(
            app.current_font_size,
            app.current_line_height,
            app.auto_adjust_line_height,
        );
        Self {
            metrics: Some(EditorMetrics {
                editor_font_size: font_size,
                editor_line_height: line_height,
                editor_auto_line_height: app.auto_adjust_line_height,
            }),
            ..Self::default()
        }
    }

    pub(super) fn theme(theme: String) -> Self {
        Self {
            editor_theme: Some(theme),
            editor_follow_system_theme: Some(false),
            ..Self::default()
        }
    }

    pub(super) fn follow_system_theme(follow: bool) -> Self {
        Self { editor_follow_system_theme: Some(follow), ..Self::default() }
    }

    fn merge(&mut self, newer: Self) {
        if newer.metrics.is_some() {
            self.metrics = newer.metrics;
        }
        if newer.editor_theme.is_some() {
            self.editor_theme = newer.editor_theme;
        }
        if newer.editor_follow_system_theme.is_some() {
            self.editor_follow_system_theme = newer.editor_follow_system_theme;
        }
    }
}

#[derive(Default)]
pub(crate) struct ConfigPersistence {
    pending: VecDeque<(GatewayEndpoint, EditorSettingsPatch)>,
    saving: bool,
    #[cfg(not(target_arch = "wasm32"))]
    shutdown_waiter: Option<tokio::sync::oneshot::Sender<()>>,
}

impl ConfigPersistence {
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn wait_for_idle(&mut self) -> Option<tokio::sync::oneshot::Receiver<()>> {
        if !self.saving {
            return None;
        }
        let (sender, receiver) = tokio::sync::oneshot::channel();
        self.shutdown_waiter = Some(sender);
        Some(receiver)
    }

    fn start_next(&mut self) -> Task<Message> {
        if self.saving {
            return Task::none();
        }
        let Some((endpoint, patch)) = self.pending.pop_front() else {
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(waiter) = self.shutdown_waiter.take() {
                let _ = waiter.send(());
            }
            return Task::none();
        };

        self.saving = true;
        Task::perform(
            async move { GatewayClient::new(endpoint)?.desktop_system_settings_patch(&patch).await },
            |result| Message::Editor(EditorMessage::ConfigSaved(result)),
        )
    }
}

fn same_gateway(left: &GatewayEndpoint, right: &GatewayEndpoint) -> bool {
    left.host == right.host
        && left.port == right.port
        && left.tenant_scope() == right.tenant_scope()
        && left.auth.as_ref().and_then(|auth| auth.skey.as_ref())
            == right.auth.as_ref().and_then(|auth| auth.skey.as_ref())
}

pub(super) fn enqueue(app: &mut App, patch: EditorSettingsPatch) -> Task<Message> {
    #[cfg(not(target_arch = "wasm32"))]
    for (edited, field) in [
        (patch.metrics.is_some(), "editor_metrics"),
        (patch.editor_theme.is_some(), "editor_theme"),
        (patch.editor_follow_system_theme.is_some(), "editor_follow_system_theme"),
    ] {
        if edited {
            app.note_startup_setting_edit(field);
        }
    }
    // Capture the active gateway at the time of the edit, even if it has to wait.
    let endpoint = gateway_client_endpoint();
    let persistence = &mut app.editor_config_persistence;
    if let Some((target, pending)) = persistence.pending.back_mut()
        && same_gateway(target, &endpoint)
    {
        pending.merge(patch);
    } else {
        persistence.pending.push_back((endpoint, patch));
    }
    persistence.start_next()
}

pub(super) fn save_complete(app: &mut App, result: Result<(), String>) -> Task<Message> {
    let persistence = &mut app.editor_config_persistence;
    persistence.saving = false;
    let next = persistence.start_next();
    if result.is_err() {
        tracing::warn!(target: "vw_desktop", "failed to save editor settings");
        return Task::batch([next, app.show_error_toast("编辑器配置保存失败，请重试")]);
    }
    next
}
