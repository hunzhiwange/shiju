//! Pi directory intent and revision handling; the SDK owns all remote access.
use crate::app::message::app_providers::list::{self, ListMessage};
use crate::app::{App, Message};
use iced::Task;
use gateway_aggregation_native::AggregationApiError;
use vw_gateway_client::GatewayClient;
use vw_api_app_identity::AppId;
use vw_api_app_aggregation::AggregationQuery;
use vw_api_app_aggregation::PiDirectoryState;
use vw_api_app_aggregation::PiDirectoryWrite;
use vw_api_transport::id::ProjectId;

pub use desktop_apps_ui::aggregation::pi::{Event, Form};
pub(crate) fn message(event: Event) -> Message {
    list::message(ListMessage::PiDirectory(event))
}
pub(crate) async fn execute(
    client: GatewayClient,
    query: AggregationQuery,
    write: Option<(String, PiDirectoryWrite)>,
) -> Result<PiDirectoryState, AggregationApiError> {
    match write {
        Some((id, body)) => gateway_aggregation_native::GatewayAggregationNativeClient::new(client.transport().clone()).pi_directory_entry(&id, &body).await,
        None => gateway_aggregation_native::GatewayAggregationNativeClient::new(client.transport().clone()).pi_directory_state(&query).await,
    }
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let Some(scope) = app.app_providers.scope.clone().filter(|s| s.app == AppId::Pi) else {
        return Task::none();
    };
    if matches!(event, Event::Close) {
        if app.app_providers.pi_directory.as_ref().is_some_and(|form| form.pending.is_none()) {
            app.app_providers.pi_directory = None;
        }
        return Task::none();
    }
    let Ok((client, actual)) = list::current(app, AppId::Pi) else {
        app.app_providers.pi_directory = None;
        return app.show_error_toast("无法连接网关");
    };
    if scope != actual {
        app.app_providers.pi_directory = None;
        return Task::none();
    }
    let Some(project) = app.project_id.as_deref().filter(|id| !id.is_empty()).map(ProjectId::from)
    else {
        app.app_providers.pi_directory = None;
        return app.show_error_toast("请先打开已注册的项目");
    };
    let access = app.apps_catalog.generation;
    if matches!(event, Event::Open) {
        let state = &mut app.app_providers;
        if state.loading
            || state.form.is_some()
            || state.editor.is_some()
            || state.pi_directory.is_some()
        {
            return Task::none();
        }
        state.pi_directory =
            Some(Form { context: Some((project.clone(), access)), ..Default::default() });
        return update(app, Event::Refresh);
    }
    let state = &mut app.app_providers;
    if state
        .pi_directory
        .as_ref()
        .is_some_and(|form| form.context.as_ref() != Some(&(project.clone(), access)))
    {
        state.pi_directory = None;
        return app.show_error_toast("项目或授权上下文已变化，请重新打开 Pi 目录");
    }
    let Some(form) = &mut state.pi_directory else { return Task::none() };
    if let Event::Completed {
        scope: previous,
        project: previous_project,
        access: previous_access,
        generation,
        result,
    } = event
    {
        if previous != scope || previous_project != project || previous_access != access {
            return Task::none();
        }
        if form.finish(generation, result)
            && let Some(error) = form.error
        {
            return app.show_error_toast(error);
        }
        return Task::none();
    }
    if form.pending.is_some() {
        return Task::none();
    }
    let query =
        AggregationQuery { target_host_id: scope.host.clone(), project_id: project.clone() };
    let write = match event {
        Event::Id(value) => {
            form.id = value;
            return Task::none();
        }
        Event::Name(value) => {
            form.name = value;
            return Task::none();
        }
        Event::Declaration(action) => {
            form.declaration.perform(action);
            return Task::none();
        }
        Event::ReviewRemove(id) => {
            form.removing = Some(id);
            return Task::none();
        }
        Event::CancelRemove => {
            form.removing = None;
            return Task::none();
        }
        Event::Refresh => {
            form.snapshot = None;
            form.removing = None;
            None
        }
        Event::Save | Event::ConfirmRemove => {
            match form.write(&query, matches!(event, Event::ConfirmRemove)) {
                Ok(write) => Some(write),
                Err(error) => return app.show_error_toast(error),
            }
        }
        _ => return Task::none(),
    };
    state.generation = state.generation.wrapping_add(1);
    let generation = state.generation;
    form.pending = Some(generation);
    form.error = None;
    Task::perform(execute(client, query, write), move |result| {
        message(Event::Completed {
            scope: scope.clone(),
            project: project.clone(),
            access,
            generation,
            result,
        })
    })
}
