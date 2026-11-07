use super::list::{self, ListMessage};
use crate::app::{
    App, Message,
    state::app_providers::{
        env::{Action, Form, Operation, SCROLL_ID, available, restore_request},
        list::Scope,
    },
};
use iced::Task;
use vw_gateway_client::{
    AgentAppsReadError, GatewayClient,
    vw_api_types::{
        agent_apps::AppId,
        app_providers::{
            env_clean::{EnvCleanRequest, EnvCleanResult},
            env_diagnose::{EnvDiagnoseRequest, EnvDiagnosis},
            env_restore::{EnvRestoreRequest, EnvRestoreResult, EnvRestoreStatus},
        },
    },
};

#[derive(Clone)]
pub enum Event {
    Open,
    Close,
    Refresh,
    Toggle(usize, bool),
    BackupId(String),
    Review(Action),
    Cancel,
    Confirm,
    Completed { scope: Scope, generation: u64, result: Result<Outcome, AgentAppsReadError> },
}

impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ProviderEnvironmentEvent([redacted])")
    }
}

#[derive(Clone)]
pub enum Outcome {
    Diagnosis(EnvDiagnosis),
    Cleaned(EnvCleanResult),
    Restored(EnvRestoreResult),
}

enum Work {
    Diagnose,
    Clean(EnvCleanRequest),
    Restore(EnvRestoreRequest),
}

pub(crate) fn message(event: Event) -> Message {
    list::message(ListMessage::Env(event))
}

pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let state = &mut app.app_providers;
    let Some(scope) = state.scope.clone() else { return Task::none() };
    if matches!(event, Event::Open) {
        if !available(scope.app)
            || state.loading
            || state.form.is_some()
            || state.editor.is_some()
            || state.env.is_some()
            || state.preview.is_some()
            || state.snippets.is_some()
            || state.universal.is_some()
            || state.icons.is_some()
            || state.endpoints.is_some()
            || state.desktop.is_some()
        {
            return Task::none();
        }
        state.env = Some(Form::default());
        return update(app, Event::Refresh);
    }
    if matches!(event, Event::Close) {
        if state.env.as_ref().is_some_and(|form| !form.writing()) {
            state.env = None;
            state.generation = state.generation.wrapping_add(1);
        }
        return Task::none();
    }

    let (client, current) = match list::current(app, scope.app) {
        Ok(context) => context,
        Err(_) => {
            app.app_providers.env = None;
            return app.show_error_toast("无法连接应用执行主机");
        }
    };
    let state = &mut app.app_providers;
    if current != scope {
        state.env = None;
        return Task::none();
    }
    let Some(form) = &mut state.env else { return Task::none() };
    let editing = form.busy.is_none() && form.confirmation.is_none();
    let work = match event {
        Event::Refresh if editing => Work::Diagnose,
        Event::Toggle(index, selected) if editing => {
            form.toggle(index, selected);
            return Task::none();
        }
        Event::BackupId(id) if editing => {
            form.backup_id = id;
            form.restored.clear();
            return Task::none();
        }
        Event::Review(action) if editing => {
            let enabled = match action {
                Action::Clean => form.can_clean(),
                Action::Restore => !form.backup_id.trim().is_empty(),
            };
            if enabled {
                form.error = None;
                form.confirmation = Some(action);
                return scroll_top();
            }
            return Task::none();
        }
        Event::Cancel if form.busy.is_none() => {
            form.confirmation = None;
            return Task::none();
        }
        Event::Confirm if form.busy.is_none() => {
            let work = match form.confirmation.take() {
                Some(Action::Clean) => form.clean_request(&scope.host).map(Work::Clean),
                Some(Action::Restore) => {
                    restore_request(&scope.host, &form.backup_id).map(Work::Restore)
                }
                None => None,
            };
            let Some(work) = work else { return Task::none() };
            work
        }
        Event::Completed { scope: completed, generation, result }
            if completed == scope && generation == state.generation && form.busy.is_some() =>
        {
            return finish(app, result, scope.app);
        }
        _ => return Task::none(),
    };
    form.error = None;
    form.busy = Some(match &work {
        Work::Diagnose => {
            form.entries.clear();
            form.selected.clear();
            Operation::Diagnose
        }
        Work::Clean(_) => Operation::Write(Action::Clean),
        Work::Restore(_) => {
            form.restored.clear();
            Operation::Write(Action::Restore)
        }
    });
    state.generation = state.generation.wrapping_add(1);
    let generation = state.generation;
    let token = app.apps_catalog.token.clone();
    let task = Task::perform(execute(client, scope.clone(), work, token), move |result| {
        message(Event::Completed { scope: scope.clone(), generation, result })
    });
    Task::batch([task, scroll_top()])
}

fn finish(
    app: &mut App,
    result: Result<Outcome, AgentAppsReadError>,
    expected_app: AppId,
) -> Task<Message> {
    let Some(form) = &mut app.app_providers.env else { return Task::none() };
    let operation = form.busy.take();
    match result {
        Ok(Outcome::Diagnosis(diagnosis)) if diagnosis.app_id == expected_app => {
            form.entries = diagnosis.entries;
            Task::none()
        }
        Ok(Outcome::Cleaned(result)) => {
            form.record_backup(result.backup_id);
            let toast = app.show_success_toast(format!(
                "已清理 {} 项，备份编号已填入。",
                result.removed_definitions
            ));
            Task::batch([toast, update(app, Event::Refresh)])
        }
        Ok(Outcome::Restored(result)) => {
            let complete = !result.items.is_empty()
                && result.items.iter().all(|item| {
                    matches!(
                        item.status,
                        EnvRestoreStatus::Restored | EnvRestoreStatus::AlreadyRestored
                    )
                });
            form.restored = result.items;
            let toast = if complete {
                app.show_success_toast("备份恢复已完成")
            } else {
                app.show_error_toast("有文件未恢复，请查看各文件的结果")
            };
            Task::batch([toast, update(app, Event::Refresh)])
        }
        result => {
            let error = match result {
                Err(AgentAppsReadError::Http { status: 401 | 403, .. }) => "执行主机拒绝了此操作。",
                Err(AgentAppsReadError::Http { status: 409, .. })
                    if matches!(operation, Some(Operation::Write(Action::Clean))) =>
                {
                    form.entries.clear();
                    form.selected.clear();
                    "环境定义已变化，请重新诊断并选择。"
                }
                Err(AgentAppsReadError::Http { status: 400, .. })
                    if matches!(operation, Some(Operation::Write(Action::Restore))) =>
                {
                    "备份编号无效，请检查后重试。"
                }
                _ => match operation {
                    Some(Operation::Write(Action::Clean)) => "无法确认清理结果，请重新诊断。",
                    Some(Operation::Write(Action::Restore)) => "无法恢复备份，请检查编号后重试。",
                    _ => "无法读取环境诊断，请重试。",
                },
            };
            form.error = Some(error);
            app.show_error_toast(error)
        }
    }
}

async fn execute(
    client: GatewayClient,
    scope: Scope,
    work: Work,
    token: String,
) -> Result<Outcome, AgentAppsReadError> {
    match work {
        Work::Diagnose => client
            .provider_env_diagnose(
                scope.app,
                &EnvDiagnoseRequest { target_host_id: scope.host },
                &token,
            )
            .await
            .map(Outcome::Diagnosis),
        Work::Clean(request) => {
            client.provider_env_clean(scope.app, &request, &token).await.map(Outcome::Cleaned)
        }
        Work::Restore(request) => {
            restore(client, scope, request, token).await.map(Outcome::Restored)
        }
    }
}

pub(super) async fn restore(
    client: GatewayClient,
    scope: Scope,
    request: EnvRestoreRequest,
    token: String,
) -> Result<EnvRestoreResult, AgentAppsReadError> {
    client.provider_env_restore(scope.app, &request, &token).await
}

fn scroll_top() -> Task<Message> {
    iced::widget::operation::snap_to(
        iced::widget::Id::new(SCROLL_ID),
        iced::widget::scrollable::RelativeOffset::START,
    )
    .map(|(): ()| Message::None)
}
