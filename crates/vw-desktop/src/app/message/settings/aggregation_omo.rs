use crate::app::{
    App, Message,
    message::app_providers::list::{self, ListMessage},
    state::{
        aggregation_omo::{Form, Snapshot, Variant},
        app_providers::list::Scope,
    },
};
use iced::Task;
use vw_gateway_client::{
    AggregationOmoError, GatewayAuth, GatewayClient,
    vw_api_types::{
        agent_apps::{
            AppId,
            aggregation::{OmoApplyRequest, OmoDisableRequest},
        },
        id::ProjectId,
    },
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Context {
    pub scope: Scope,
    pub project: ProjectId,
    pub access: u64,
}
#[derive(Clone)]
pub enum Event {
    Open(Variant),
    Close,
    Refresh,
    Provider(String),
    Apply,
    Disable,
    Finished { context: Context, generation: u64, result: Result<Snapshot, AggregationOmoError> },
}
impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OmoEvent")
    }
}
pub(crate) fn message(event: Event) -> Message {
    list::message(ListMessage::Omo(event))
}
fn current(app: &App) -> Result<(GatewayClient, Context), String> {
    let (client, scope) = list::current(app, AppId::OpenCode)?;
    let project =
        app.project_id.as_ref().filter(|id| !id.is_empty()).ok_or("请先打开已注册项目")?;
    if app.app_providers.scope.as_ref() != Some(&scope)
        || scope.host.is_empty()
        || app.apps_catalog.token.is_empty()
    {
        return Err("请先连接应用执行主机".into());
    }
    let endpoint = client
        .endpoint()
        .clone()
        .with_auth(GatewayAuth { skey: Some(app.apps_catalog.token.clone()) });
    Ok((
        GatewayClient::new(endpoint)?,
        Context { scope, project: project.as_str().into(), access: app.apps_catalog.generation },
    ))
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Action {
    Read,
    Apply(String),
    Disable,
}
pub(crate) async fn request(
    client: GatewayClient,
    context: Context,
    variant: Variant,
    action: Action,
    revision: String,
) -> Result<Snapshot, AggregationOmoError> {
    let host = context.scope.host;
    let project = context.project;
    match action {
        Action::Read => match variant {
            Variant::Standard => {
                client.aggregation_omo_state(&host, &project).await.map(Snapshot::Standard)
            }
            Variant::Slim => {
                client.aggregation_omo_slim_state(&host, &project).await.map(Snapshot::Slim)
            }
        },
        Action::Apply(provider_id) => {
            let request = OmoApplyRequest {
                target_host_id: host,
                project_id: project,
                expected_revision: revision,
                provider_id,
            };
            match variant {
                Variant::Standard => {
                    client.aggregation_omo_apply(&request).await.map(Snapshot::Standard)
                }
                Variant::Slim => {
                    client.aggregation_omo_slim_apply(&request).await.map(Snapshot::Slim)
                }
            }
        }
        Action::Disable => {
            let request = OmoDisableRequest {
                target_host_id: host,
                project_id: project,
                expected_revision: revision,
            };
            match variant {
                Variant::Standard => {
                    client.aggregation_omo_disable(&request).await.map(Snapshot::Standard)
                }
                Variant::Slim => {
                    client.aggregation_omo_slim_disable(&request).await.map(Snapshot::Slim)
                }
            }
        }
    }
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    if matches!(event, Event::Close) {
        // Closing detaches the view; it does not claim to cancel a target transaction.
        app.app_providers.omo = None;
        app.app_providers.generation = app.app_providers.generation.wrapping_add(1);
        return Task::none();
    }
    let (client, context) = match current(app) {
        Ok(v) => v,
        Err(_) if matches!(event, Event::Finished { .. }) => return Task::none(),
        Err(e) => return app.show_error_toast(e),
    };
    if let Event::Finished { context: original, generation, result } = event {
        if original != context || generation != app.app_providers.generation {
            return Task::none();
        }
        let Some(form) = app.app_providers.omo.as_mut().filter(|f| f.busy) else {
            return Task::none();
        };
        form.finish(result);
        return match form.error.clone() {
            Some(e) => app.show_error_toast(e),
            None => app.show_info_toast("已读取目标实际状态"),
        };
    }
    if let Event::Open(variant) = event {
        if app.app_providers.omo.is_some() || app.app_providers.loading {
            return Task::none();
        }
        let mut form = Form::new(variant);
        form.binding = Some((context.project.clone(), context.access));
        app.app_providers.omo = Some(form);
    } else if app.app_providers.omo.as_ref().is_none_or(|f| f.busy) {
        return Task::none();
    }
    if app
        .app_providers
        .omo
        .as_ref()
        .is_some_and(|f| f.binding.as_ref() != Some(&(context.project.clone(), context.access)))
    {
        app.app_providers.omo = None;
        return app.show_error_toast("项目或授权已切换，请重新打开目录");
    }
    let Some(form) = app.app_providers.omo.as_mut() else { return Task::none() };
    if let Event::Provider(v) = event {
        form.provider = v;
        return Task::none();
    }
    let revision = form.snapshot.as_ref().map(|s| s.revision().to_owned()).unwrap_or_default();
    let action = match event {
        Event::Apply => {
            let saved = app
                .app_providers
                .snapshot
                .as_ref()
                .is_some_and(|s| s.catalog.rows.iter().any(|r| r.provider.id == form.provider));
            if revision.is_empty() || !saved {
                return app.show_error_toast("请选择已保存供应商，并刷新配置状态");
            }
            Action::Apply(form.provider.clone())
        }
        Event::Disable if !revision.is_empty() => Action::Disable,
        Event::Open(_) | Event::Refresh => Action::Read,
        _ => return Task::none(),
    };
    form.busy = true;
    let variant = form.variant;
    app.app_providers.generation = app.app_providers.generation.wrapping_add(1);
    let generation = app.app_providers.generation;
    Task::perform(request(client, context.clone(), variant, action, revision), move |result| {
        message(Event::Finished { context: context.clone(), generation, result })
    })
}
#[cfg(test)]
#[path = "aggregation_omo_tests.rs"]
mod tests;
