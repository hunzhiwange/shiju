//! Poll and cancel discoveries on the gateway that accepted the request.

use super::catalog_api::{self, Event as CatalogEvent};
use crate::app::{
    App, Message,
    state::app_providers::model_catalog::{CatalogForm, DiscoveryPhase},
};
use iced::{
    Task,
    futures::{SinkExt, channel::mpsc},
};
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};
use tokio::sync::watch;
use vw_gateway_client::{
    AgentAppsReadError, GatewayClient,
    vw_api_types::{
        id::TaskId,
        model_catalog::{
            ModelCatalogProject, ModelDiscoveryMethod, ModelDiscoveryRequest, ModelDiscoveryState,
            ModelDiscoveryStatus,
        },
    },
};

#[derive(Clone)]
pub enum Event {
    Start(ModelDiscoveryMethod),
    Cancel,
    Toggle { run: u64, index: usize, checked: bool },
    Status { run: u64, phase: DiscoveryPhase },
}

impl Event {
    pub(super) fn is_response(&self) -> bool {
        matches!(self, Self::Status { .. })
    }
}

pub(super) fn update(app: &mut App, client: GatewayClient, event: Event) -> Task<Message> {
    let Some(form) = &mut app.app_providers.model_catalog else { return Task::none() };
    match event {
        Event::Start(method) if !form.busy() && !form.discovery.phase.active() => {
            let deployment = form.deployment();
            if deployment.endpoint.is_empty() || !methods(form).contains(&method) {
                return Task::none();
            }
            static NEXT_RUN: AtomicU64 = AtomicU64::new(1);
            let run = NEXT_RUN.fetch_add(1, Ordering::Relaxed);
            let (cancel, receiver) = watch::channel(false);
            form.discovery.run = run;
            form.discovery.method = Some(method);
            form.discovery.cancel = Some(cancel);
            form.discovery.phase = DiscoveryPhase::Starting;
            let worker = Worker {
                client,
                token: app.apps_catalog.token.clone(),
                project: form.context.project.clone(),
                request: ModelDiscoveryRequest { deployment, method, account_id: None },
                context: form.context.scope.context.tenant.clone(),
            };
            let stream = iced::stream::channel(1, async move |mut output| {
                let phase = worker.run(receiver, &mut output).await;
                let _ = output.send(phase).await;
            });
            return Task::run(stream, move |phase| {
                catalog_api::message(CatalogEvent::Discovery(Event::Status { run, phase }))
            });
        }
        Event::Cancel if form.discovery.phase.running() => {
            form.discovery.phase = match &form.discovery.cancel {
                Some(cancel) if cancel.send(true).is_ok() => DiscoveryPhase::Stopping,
                _ => {
                    form.discovery.cancel = None;
                    DiscoveryPhase::Cancelled
                }
            };
        }
        Event::Status { run, phase }
            if form.discovery.run == run && form.discovery.phase.active() =>
        {
            if matches!(form.discovery.phase, DiscoveryPhase::Stopping) {
                if phase.active() {
                    return Task::none();
                }
                // Even if completion won on the server, cancellation has already
                // won in this view. Never publish those results.
                form.discovery.phase = if matches!(phase, DiscoveryPhase::CancelUnconfirmed) {
                    phase
                } else {
                    DiscoveryPhase::Cancelled
                };
            } else {
                if let DiscoveryPhase::Completed(models) = &phase {
                    form.discovery.results = models.clone();
                    form.discovery.results_run = run;
                }
                form.discovery.phase = phase;
            }
            if !form.discovery.phase.active() {
                form.discovery.cancel = None;
            }
        }
        Event::Toggle { run, index, checked }
            if !form.busy() && run == form.discovery.results_run =>
        {
            if let Some(identity) = form.discovery.results.get(index).cloned() {
                form.toggle_identity(identity, checked);
            }
        }
        _ => {}
    }
    Task::none()
}

pub(crate) fn methods(form: &CatalogForm) -> &'static [ModelDiscoveryMethod] {
    use ModelDiscoveryMethod::{Agy, Http, OpenCode};
    use vw_gateway_client::vw_api_types::agent_apps::AppId;
    match form.context.scope.app {
        AppId::OpenCode => &[OpenCode, Http],
        AppId::Agy => &[Agy, Http],
        _ => &[Http],
    }
}

struct Worker {
    client: GatewayClient,
    token: String,
    project: ModelCatalogProject,
    request: ModelDiscoveryRequest,
    context: vw_gateway_client::vw_api_types::id::TenantScope,
}

impl Worker {
    async fn run(
        &self,
        mut cancel: watch::Receiver<bool>,
        output: &mut mpsc::Sender<DiscoveryPhase>,
    ) -> DiscoveryPhase {
        // A cancelled POST can still create a task. Wait for the ID so that
        // closing the modal or cancelling while submitting can issue DELETE.
        let mut status = match self
            .client
            .start_model_discovery(&self.project, &self.request, &self.token)
            .await
        {
            Ok(status) if status.deployment == self.request.deployment => status,
            Ok(_) => return DiscoveryPhase::Failed("网关返回了其他部署的发现任务。".into()),
            Err(error) => return request_failed(error),
        };
        let id = status.task_id.clone();
        loop {
            if *cancel.borrow() || cancel.has_changed().is_err() {
                return self.stop(&id).await;
            }
            let phase = match status.state {
                ModelDiscoveryState::Queued => DiscoveryPhase::Queued,
                ModelDiscoveryState::Discovering => DiscoveryPhase::Discovering,
                ModelDiscoveryState::Parsing => DiscoveryPhase::Parsing,
                ModelDiscoveryState::Cancelled => return DiscoveryPhase::Cancelled,
                ModelDiscoveryState::Failed { reason } => {
                    return DiscoveryPhase::ServiceFailed(reason);
                }
                ModelDiscoveryState::Completed { models } => {
                    if models.iter().any(|row| {
                        row.identity.scope != self.context
                            || row.identity.deployment != self.request.deployment
                    }) {
                        return DiscoveryPhase::Failed("网关返回了其他部署的发现结果。".into());
                    }
                    return DiscoveryPhase::Completed(
                        models.into_iter().map(|row| row.identity).collect(),
                    );
                }
            };
            if output.send(phase).await.is_err() {
                return self.stop(&id).await;
            }
            let response = tokio::select! {
                biased;
                _ = cancel.changed() => return self.stop(&id).await,
                response = async {
                    tokio::time::sleep(Duration::from_millis(300)).await;
                    self.client.model_discovery(
                        &self.project, &self.request.deployment.target_host_id, &id, &self.token,
                    ).await
                } => response,
            };
            status = match response {
                Ok(status) if self.matches(&id, &status) => status,
                Ok(_) => {
                    let _ = self.stop(&id).await;
                    return DiscoveryPhase::Failed("网关返回了不匹配的发现任务。".into());
                }
                Err(error) => {
                    let _ = self.stop(&id).await;
                    return request_failed(error);
                }
            };
        }
    }

    fn matches(&self, id: &TaskId, status: &ModelDiscoveryStatus) -> bool {
        &status.task_id == id && status.deployment == self.request.deployment
    }

    async fn stop(&self, id: &TaskId) -> DiscoveryPhase {
        match self
            .client
            .cancel_model_discovery(
                &self.project,
                &self.request.deployment.target_host_id,
                id,
                &self.token,
            )
            .await
        {
            Ok(status)
                if self.matches(id, &status)
                    && matches!(
                        status.state,
                        ModelDiscoveryState::Cancelled
                            | ModelDiscoveryState::Completed { .. }
                            | ModelDiscoveryState::Failed { .. }
                    ) =>
            {
                DiscoveryPhase::Cancelled
            }
            _ => DiscoveryPhase::CancelUnconfirmed,
        }
    }
}

fn request_failed(error: AgentAppsReadError) -> DiscoveryPhase {
    let label = match error {
        AgentAppsReadError::Http { status: 401 | 403, .. } => {
            "发现请求被拒绝，请检查执行主机授权和供应商保存的端点。".into()
        }
        AgentAppsReadError::Http { status: 422, .. } => {
            "此部署无法使用所选发现方式，请检查已保存配置或执行主机的应用。".into()
        }
        AgentAppsReadError::Http { status, .. } => format!("发现请求失败（HTTP {status}）。"),
        _ => "发现请求失败，请检查网关连接后重试。".into(),
    };
    DiscoveryPhase::Failed(label)
}
