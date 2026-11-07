//! 两个路由设置页共用有效快照、停顿合并和单个在途 PATCH。

use super::messages::{ModelRoutesMessage, SettingsMessage};
use crate::app::config::gateway_client_endpoint;
use crate::app::{App, Message};
use iced::Task;
use serde_json::Value;
use std::collections::VecDeque;
use vw_gateway_client::{GatewayClient, GatewayEndpoint};

#[derive(Debug, Clone)]
struct Pending {
    endpoint: GatewayEndpoint,
    revision: u64,
    patch: Value,
    ready: bool,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct RoutingPersistence {
    pub(super) revision: u64,
    pub(super) dirty: bool,
    pub(super) route_invalid: bool,
    pub(super) classification_invalid: bool,
    failed: bool,
    loading_endpoint: Option<GatewayEndpoint>,
    pending: VecDeque<Pending>,
    saving: Option<Pending>,
    #[cfg(not(target_arch = "wasm32"))]
    idle: Option<tokio::sync::watch::Sender<bool>>,
}

fn message(message: ModelRoutesMessage) -> Message {
    Message::Settings(SettingsMessage::ModelRoutes(message))
}

fn same_gateway(left: &GatewayEndpoint, right: &GatewayEndpoint) -> bool {
    left.host == right.host
        && left.port == right.port
        && left.tenant_scope() == right.tenant_scope()
        && left.auth.as_ref().and_then(|auth| auth.skey.as_ref())
            == right.auth.as_ref().and_then(|auth| auth.skey.as_ref())
}

// Only three top-level domains are queued. Classification is replaced as a pair;
// classification-only edits must retain the pending app_ui and model_routes fields.
fn merge(older: &mut Value, newer: Value) {
    older
        .as_object_mut()
        .expect("routing patch object")
        .extend(newer.as_object().expect("routing patch object").clone());
}

impl RoutingPersistence {
    pub(super) fn load_is_current(&self) -> bool {
        self.loading_endpoint
            .as_ref()
            .is_some_and(|endpoint| same_gateway(endpoint, &gateway_client_endpoint()))
    }

    pub(super) fn edited(&mut self) {
        self.revision += 1;
        self.dirty = true;
    }

    fn start_next(&mut self) -> Task<Message> {
        if self.saving.is_some() || !self.pending.front().is_some_and(|pending| pending.ready) {
            return Task::none();
        }
        let pending = self.pending.pop_front().expect("ready pending patch");
        let endpoint = pending.endpoint.clone();
        let patch = pending.patch.clone();
        let revision = pending.revision;
        self.saving = Some(pending);
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(idle) = &self.idle {
            idle.send_replace(false);
        }
        Task::perform(
            async move { GatewayClient::new(endpoint)?.global_config_patch(&patch).await },
            move |result| message(ModelRoutesMessage::Saved(revision, result)),
        )
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn flush(&mut self) -> (Task<Message>, Option<tokio::sync::watch::Receiver<bool>>) {
        if self.pending.is_empty() && self.saving.is_none() {
            return (Task::none(), None);
        }
        for pending in &mut self.pending {
            pending.ready = true;
        }
        let (sender, receiver) = tokio::sync::watch::channel(false);
        self.idle = Some(sender);
        (self.start_next(), Some(receiver))
    }
}

pub(super) fn enqueue(app: &mut App, patch: Value) -> Task<Message> {
    let endpoint = gateway_client_endpoint();
    let state = &mut app.model_routes_settings.persistence;
    let revision = state.revision;
    state.failed = false;
    if let Some(pending) = state.pending.back_mut()
        && same_gateway(&pending.endpoint, &endpoint)
    {
        merge(&mut pending.patch, patch);
        pending.revision = revision;
        pending.ready = false;
    } else {
        state.pending.push_back(Pending { endpoint, revision, patch, ready: false });
    }
    crate::app::message::after(
        std::time::Duration::from_millis(300),
        message(ModelRoutesMessage::SaveDue(revision)),
    )
}

pub(super) fn save_due(app: &mut App, revision: u64) -> Task<Message> {
    let state = &mut app.model_routes_settings.persistence;
    if let Some(pending) = state.pending.iter_mut().find(|pending| pending.revision == revision) {
        pending.ready = true;
    }
    state.start_next()
}

pub(super) fn saved(app: &mut App, revision: u64, result: Result<(), String>) -> Task<Message> {
    let state = &mut app.model_routes_settings.persistence;
    if !state.saving.as_ref().is_some_and(|saving| saving.revision == revision) {
        return Task::none();
    }
    let mut saved = state.saving.take().expect("matching save");
    let current_gateway = same_gateway(&saved.endpoint, &gateway_client_endpoint());
    state.failed = result.is_err();
    if result.is_err() {
        // Keep failed fields for the next valid edit or explicit refresh. A newer
        // classification selection wins, including when the failed PATCH had routes.
        if let Some(next) = state.pending.front_mut()
            && same_gateway(&saved.endpoint, &next.endpoint)
        {
            merge(&mut saved.patch, next.patch.clone());
            next.patch = saved.patch;
        } else {
            saved.ready = false;
            state.pending.push_front(saved);
        }
    } else if state.revision == revision
        && !state.route_invalid
        && !state.classification_invalid
        && current_gateway
    {
        state.dirty = false;
        app.model_routes_settings.save_error = None;
        app.query_classification_settings.save_error = None;
        app.query_classification_settings.save_error_translation = None;
    }
    let next = state.start_next();
    #[cfg(not(target_arch = "wasm32"))]
    if state.saving.is_none()
        && !state.pending.iter().any(|pending| pending.ready)
        && let Some(idle) = &state.idle
    {
        idle.send_replace(true);
    }
    if let Err(error) = result {
        if state.revision == revision && current_gateway {
            let error = crate::app::config::server_config_unreachable_error(error);
            if !state.route_invalid {
                app.model_routes_settings.save_error = Some(error.clone().into());
            }
            if !state.classification_invalid {
                app.query_classification_settings.save_error = Some(error);
                app.query_classification_settings.save_error_translation = None;
            }
        }
        return Task::batch([next, app.show_error_toast("路由分类配置保存失败，请重试")]);
    }
    next
}

pub(super) fn refresh(app: &mut App) -> Task<Message> {
    let state = &mut app.model_routes_settings.persistence;
    if state.failed {
        state.failed = false;
        for pending in &mut state.pending {
            pending.ready = true;
        }
        return state.start_next();
    }
    // Switching tabs must not discard either an invalid draft or queued valid edits.
    if state.dirty || state.saving.is_some() || !state.pending.is_empty() {
        return Task::none();
    }
    state.revision += 1;
    let revision = state.revision;
    let endpoint = gateway_client_endpoint();
    state.loading_endpoint = Some(endpoint.clone());
    Task::perform(
        async move {
            let config = GatewayClient::new(endpoint)?.global_config_get().await?;
            let routes = serde_json::from_value(
                config.get("model_routes").cloned().unwrap_or_else(|| serde_json::json!([])),
            )
            .map_err(|error| error.to_string())?;
            let classification = serde_json::from_value(
                config
                    .get("query_classification")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({})),
            )
            .map_err(|error| error.to_string())?;
            Ok((routes, classification))
        },
        move |result| message(ModelRoutesMessage::Loaded(revision, result)),
    )
}

#[cfg(test)]
#[path = "routing_persistence_tests.rs"]
mod tests;
