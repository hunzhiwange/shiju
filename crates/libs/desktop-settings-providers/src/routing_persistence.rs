//! 两个路由设置页共用有效快照、停顿合并和单个在途 PATCH。

use desktop_config::ConfigAccess;
use config_routing::{ModelRouteConfig, QueryClassificationConfig};
#[derive(Debug, Clone)]
pub enum Message {
    Saved(u64, Result<(), String>),
    Loaded(u64, Result<(Vec<ModelRouteConfig>, QueryClassificationConfig), String>),
}

/// Only accepted completions produce feedback; stale responses have no effects.
pub struct SaveOutcome {
    pub task: Task<Message>,
    pub clear_errors: bool,
    pub error: Option<String>,
    pub show_error: bool,
}
use iced::Task;
use serde_json::Value;
use std::collections::VecDeque;
use gateway_endpoint::GatewayEndpoint;

#[derive(Debug, Clone)]
struct Pending {
    endpoint: GatewayEndpoint,
    revision: u64,
    patch: Value,
    ready: bool,
}

#[derive(Debug, Clone, Default)]
pub struct RoutingPersistence {
    pub revision: u64,
    pub dirty: bool,
    pub route_invalid: bool,
    pub classification_invalid: bool,
    failed: bool,
    loading_endpoint: Option<GatewayEndpoint>,
    pending: VecDeque<Pending>,
    saving: Option<Pending>,
    #[cfg(not(target_arch = "wasm32"))]
    idle: Option<tokio::sync::watch::Sender<bool>>,
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
    pub fn load_is_current(&self, current: &GatewayEndpoint) -> bool {
        self.loading_endpoint
            .as_ref()
            .is_some_and(|endpoint| same_gateway(endpoint, current))
    }

    pub fn edited(&mut self) {
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
            async move { ConfigAccess::new(endpoint, None).patch_full_agent_config_async(patch).await },
            move |result| Message::Saved(revision, result),
        )
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn flush(&mut self) -> (Task<Message>, Option<tokio::sync::watch::Receiver<bool>>) {
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

pub fn enqueue(state: &mut RoutingPersistence, endpoint: GatewayEndpoint, patch: Value) -> u64 {
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
    revision
}

pub fn save_due(state: &mut RoutingPersistence, revision: u64) -> Task<Message> {
    if let Some(pending) = state.pending.iter_mut().find(|pending| pending.revision == revision) {
        pending.ready = true;
    }
    state.start_next()
}

pub fn saved(state: &mut RoutingPersistence, current: &GatewayEndpoint, revision: u64, result: Result<(), String>) -> SaveOutcome {
    if !state.saving.as_ref().is_some_and(|saving| saving.revision == revision) {
        return SaveOutcome { task: Task::none(), clear_errors: false, error: None, show_error: false };
    }
    let mut saved = state.saving.take().expect("matching save");
    let current_gateway = same_gateway(&saved.endpoint, current);
    state.failed = result.is_err();
    let mut clear_errors = false;
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
        clear_errors = true;
    }
    let next = state.start_next();
    #[cfg(not(target_arch = "wasm32"))]
    if state.saving.is_none()
        && !state.pending.iter().any(|pending| pending.ready)
        && let Some(idle) = &state.idle
    {
        idle.send_replace(true);
    }
    let show_error = result.is_err();
    let error = result.err().filter(|_| state.revision == revision && current_gateway);
    SaveOutcome { task: next, clear_errors, error, show_error }
}

pub fn refresh(state: &mut RoutingPersistence, endpoint: GatewayEndpoint) -> Task<Message> {
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
    state.loading_endpoint = Some(endpoint.clone());
    Task::perform(
        async move {
            let config = ConfigAccess::new(endpoint, None).load_global_config_value_async().await?;
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
        move |result| Message::Loaded(revision, result),
    )
}
