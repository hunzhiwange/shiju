// OMP directory actions use the existing application scope and typed SDK.
pub use desktop_apps_ui::aggregation::omp_core::*;
use vw_gateway_client::GatewayClient;

use iced::Task;
use vw_api_app_aggregation::AggregationQuery;
use vw_api_transport::id::ProjectId;
pub use desktop_apps_ui::aggregation::omp::{Event};
fn message(event: Event) -> Event { event }
pub fn update(
    directory: &mut Option<Form>,
    counter: &mut u64,
    loading: bool,
    form_open: bool,
    editor_open: bool,
    client: GatewayClient,
    scope: desktop_apps_ui::ProviderScope,
    project: ProjectId,
    access: u64,
    notice: &mut Option<super::Notice>,
    event: Event,
) -> Task<Event> {
    if matches!(event, Event::Open) {
        if loading
            || form_open
            || editor_open
            || (*directory).is_some()
        {
            return Task::none();
        }
        (*directory) = Some(Form { project: Some(project.clone()), access, ..Form::default() });
        return update(directory, counter, loading, form_open, editor_open, client, scope, project, access, notice, Event::Refresh);
    }
    let Some(form) = &mut (*directory) else { return Task::none() };
    if form.project.as_ref() != Some(&project) || form.access != access {
        (*directory) = None;
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
            return super::error(notice, error);
        }
        return if writing {
            super::success(notice, "OMP 独立条目已更新")
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
            (*directory) = None;
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
                Err(error) => return super::error(notice, error),
            }
        }
        _ => return Task::none(),
    };
    *counter = counter.wrapping_add(1);
    let generation = *counter;
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

#[cfg(test)]
#[path = "omp_tests.rs"]
mod omp_tests;
