// Pi directory intent and revision handling; the SDK owns all remote access.
use iced::Task;
use gateway_aggregation_native::AggregationApiError;
use vw_gateway_client::GatewayClient;
use vw_api_app_aggregation::AggregationQuery;
use vw_api_app_aggregation::PiDirectoryState;
use vw_api_app_aggregation::PiDirectoryWrite;
use vw_api_transport::id::ProjectId;

pub use desktop_apps_ui::aggregation::pi::{Event, Form};
fn message(event: Event) -> Event { event }
pub async fn execute(
    client: GatewayClient,
    query: AggregationQuery,
    write: Option<(String, PiDirectoryWrite)>,
) -> Result<PiDirectoryState, AggregationApiError> {
    match write {
        Some((id, body)) => gateway_aggregation_native::GatewayAggregationNativeClient::new(client.transport().clone()).pi_directory_entry(&id, &body).await,
        None => gateway_aggregation_native::GatewayAggregationNativeClient::new(client.transport().clone()).pi_directory_state(&query).await,
    }
}
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
        (*directory) =
            Some(Form { context: Some((project.clone(), access)), ..Default::default() });
        return update(directory, counter, loading, form_open, editor_open, client, scope, project, access, notice, Event::Refresh);
    }
    if (*directory)
        .as_ref()
        .is_some_and(|form| form.context.as_ref() != Some(&(project.clone(), access)))
    {
        (*directory) = None;
        return super::error(notice, "项目或授权上下文已变化，请重新打开 Pi 目录");
    }
    let Some(form) = &mut (*directory) else { return Task::none() };
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
            return super::error(notice, error);
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
                Err(error) => return super::error(notice, error),
            }
        }
        _ => return Task::none(),
    };
    *counter = counter.wrapping_add(1);
    let generation = *counter;
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

#[cfg(test)]
#[path = "pi_tests.rs"]
mod tests;
