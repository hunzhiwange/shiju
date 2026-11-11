//! OMP directory actions use the existing application scope and typed SDK.
mod core;
use crate::app::{
    App, Message,
    message::app_providers::list::{self, ListMessage},
    state::app_providers::list::Scope,
};
pub(crate) use core::*;
use iced::{Task, widget::text_editor};
use vw_gateway_client::vw_api_types::{
    agent_apps::{AppId, aggregation::AggregationQuery},
    id::ProjectId,
};
#[derive(Clone)]
pub enum Event {
    Open,
    Close,
    Refresh,
    Id(String),
    Name(String),
    Declaration(text_editor::Action),
    Save,
    ReviewRemove(String),
    CancelRemove,
    ConfirmRemove,
    Completed {
        scope: Scope,
        project: ProjectId,
        access: u64,
        generation: u64,
        writing: bool,
        result: Result<
            vw_gateway_client::vw_api_types::agent_apps::aggregation::OmpDirectoryState,
            String,
        >,
    },
}
impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OmpDirectoryEvent([redacted])")
    }
}
pub(crate) fn message(event: Event) -> Message {
    list::message(ListMessage::OmpDirectory(event))
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let Some(scope) = app.app_providers.scope.clone().filter(|s| s.app == AppId::Omp) else {
        return Task::none();
    };
    let Ok((client, actual)) = list::current(app, AppId::Omp) else {
        return app.show_error_toast("无法连接网关");
    };
    if scope != actual {
        app.app_providers.omp_directory = None;
        return Task::none();
    }
    let Some(project) = app.project_id.as_deref().filter(|s| !s.is_empty()).map(ProjectId::from)
    else {
        app.app_providers.omp_directory = None;
        return app.show_error_toast("请先打开已注册的项目");
    };
    let access = app.apps_catalog.generation;
    if matches!(event, Event::Open) {
        let state = &mut app.app_providers;
        if state.loading
            || state.form.is_some()
            || state.editor.is_some()
            || state.omp_directory.is_some()
        {
            return Task::none();
        }
        state.omp_directory = Some(Form { project: Some(project), access, ..Form::default() });
        return update(app, Event::Refresh);
    }
    let state = &mut app.app_providers;
    let Some(form) = &mut state.omp_directory else { return Task::none() };
    if form.project.as_ref() != Some(&project) || form.access != access {
        state.omp_directory = None;
        return Task::none();
    }
    if let Event::Completed {
        scope: previous,
        project: previous_project,
        access: previous_access,
        generation,
        writing,
        result,
    } = event
    {
        if previous != scope || previous_project != project || previous_access != access {
            return Task::none();
        }
        if !form.finish(generation, result) {
            return Task::none();
        }
        if let Some(error) = form.error.clone() {
            return app.show_error_toast(error);
        }
        return if writing {
            app.show_success_toast("OMP 独立条目已更新")
        } else {
            Task::none()
        };
    }
    if form.pending.is_some() {
        return Task::none();
    }
    let query =
        AggregationQuery { target_host_id: scope.host.clone(), project_id: project.clone() };
    let write = match event {
        Event::Close => {
            state.omp_directory = None;
            return Task::none();
        }
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
            form.state = None;
            form.removing = None;
            None
        }
        Event::Save | Event::ConfirmRemove => {
            match form.write(&query, matches!(event, Event::ConfirmRemove)) {
                Ok(body) => Some(body),
                Err(error) => return app.show_error_toast(error),
            }
        }
        _ => return Task::none(),
    };
    state.generation = state.generation.wrapping_add(1);
    let generation = state.generation;
    form.pending = Some(generation);
    form.error = None;
    let writing = write.is_some();
    Task::perform(execute(client, query, write), move |result| {
        message(Event::Completed {
            scope: scope.clone(),
            project: project.clone(),
            access,
            generation,
            writing,
            result,
        })
    })
}
