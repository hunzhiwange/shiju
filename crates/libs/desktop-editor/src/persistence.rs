//! 合并编辑器设置补丁，并在关停前排空已提交的保存请求。

use desktop_config::config_system_settings::normalize_editor_metrics;
use iced::Task;
use serde::Serialize;
use std::collections::VecDeque;
use gateway_endpoint::GatewayEndpoint;
use vw_gateway_client::GatewayClient;

use crate::message::editor::EditorMessage;

#[derive(Serialize)]
struct EditorMetrics {
    editor_font_size: f32,
    editor_line_height: f32,
    editor_auto_line_height: bool,
}

#[derive(Default, Serialize)]
pub struct EditorSettingsPatch {
    #[serde(flatten)]
    metrics: Option<EditorMetrics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    editor_theme: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    editor_follow_system_theme: Option<bool>,
}

impl EditorSettingsPatch {
    pub fn metrics(font_size: f32, line_height: f32, auto: bool) -> Self {
        let (font_size, line_height) = normalize_editor_metrics(font_size, line_height, auto);
        Self {
            metrics: Some(EditorMetrics {
                editor_font_size: font_size,
                editor_line_height: line_height,
                editor_auto_line_height: auto,
            }),
            ..Self::default()
        }
    }

    pub fn theme(theme: String) -> Self {
        Self {
            editor_theme: Some(theme),
            editor_follow_system_theme: Some(false),
            ..Self::default()
        }
    }

    pub fn follow_system_theme(follow: bool) -> Self {
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
pub struct ConfigPersistence {
    pending: VecDeque<(GatewayEndpoint, EditorSettingsPatch)>,
    saving: bool,
    #[cfg(not(target_arch = "wasm32"))]
    shutdown_waiter: Option<tokio::sync::oneshot::Sender<()>>,
}

impl ConfigPersistence {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn wait_for_idle(&mut self) -> Option<tokio::sync::oneshot::Receiver<()>> {
        if !self.saving {
            return None;
        }
        let (sender, receiver) = tokio::sync::oneshot::channel();
        self.shutdown_waiter = Some(sender);
        Some(receiver)
    }

    fn start_next(&mut self) -> Task<EditorMessage> {
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
            async move {
                gateway_desktop_settings::GatewayDesktopSettingsClient::new(
                    GatewayClient::new(endpoint)?.transport().clone(),
                )
                .desktop_system_settings_patch(&patch)
                .await
            },
            EditorMessage::ConfigSaved,
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

impl EditorSettingsPatch {
    pub fn edited_fields(&self) -> impl Iterator<Item = &'static str> {
        [
            (self.metrics.is_some(), "editor_metrics"),
            (self.editor_theme.is_some(), "editor_theme"),
            (self.editor_follow_system_theme.is_some(), "editor_follow_system_theme"),
        ].into_iter().filter_map(|(edited, field)| edited.then_some(field))
    }
}

impl ConfigPersistence {
    pub fn enqueue(&mut self, endpoint: GatewayEndpoint, patch: EditorSettingsPatch) -> Task<EditorMessage> {
        if let Some((target, pending)) = self.pending.back_mut()
            && same_gateway(target, &endpoint)
        {
            pending.merge(patch);
        } else {
            self.pending.push_back((endpoint, patch));
        }
        self.start_next()
    }

    pub fn save_complete(&mut self) -> Task<EditorMessage> {
        self.saving = false;
        self.start_next()
    }
}
