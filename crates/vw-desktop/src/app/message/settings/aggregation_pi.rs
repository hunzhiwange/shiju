//! Pi directory intent and revision handling; the SDK owns all remote access.
use crate::app::message::app_providers::list::{self, ListMessage};
use crate::app::state::app_providers::list::Scope;
use crate::app::{App, Message};
use iced::{Task, widget::text_editor};
use vw_gateway_client::{
    AggregationApiError, GatewayClient,
    vw_api_types::{
        agent_apps::{
            AppId,
            aggregation::{
                AggregationQuery, PiDirectoryChange, PiDirectoryState, PiDirectoryWrite,
            },
        },
        app_providers::pi::PiProfile,
        id::ProjectId,
    },
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
        result: Result<PiDirectoryState, AggregationApiError>,
    },
}
#[derive(Clone, Default)]
pub(crate) struct Form {
    pub context: Option<(ProjectId, u64)>,
    pub snapshot: Option<PiDirectoryState>,
    pub id: String,
    pub name: String,
    pub declaration: text_editor::Content,
    pub removing: Option<String>,
    pub pending: Option<u64>,
    pub error: Option<&'static str>,
}
impl Form {
    pub(crate) fn write(
        &self,
        query: &AggregationQuery,
        remove: bool,
    ) -> Result<(String, PiDirectoryWrite), &'static str> {
        let state =
            self.snapshot.as_ref().filter(|s| !s.revision.is_empty()).ok_or("请先刷新目录")?;
        let id = if remove {
            self.removing.as_deref().ok_or("请先确认删除条目")?
        } else {
            self.id.trim()
        };
        if id.is_empty()
            || id.len() > 128
            || !id.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        {
            return Err("条目 ID 仅支持 ASCII 字母、数字、下划线和连字符");
        }
        let change = if remove {
            if !state.members.iter().any(|member| member == id) {
                return Err("条目已变化，请刷新目录");
            }
            PiDirectoryChange::Remove {}
        } else {
            if self.name.trim().is_empty() {
                return Err("请填写名称");
            }
            let declaration = serde_json::from_str(&self.declaration.text())
                .map_err(|_| "模型声明必须为 JSON 对象")?;
            if !matches!(declaration, serde_json::Value::Object(_)) {
                return Err("模型声明必须为 JSON 对象");
            }
            let profile = PiProfile { name: self.name.trim().into(), declaration };
            if state.members.iter().any(|member| member == id) {
                PiDirectoryChange::Replace { profile }
            } else {
                PiDirectoryChange::Insert { profile }
            }
        };
        Ok((
            id.into(),
            PiDirectoryWrite {
                target_host_id: query.target_host_id.clone(),
                project_id: query.project_id.clone(),
                expected_revision: state.revision.clone(),
                change,
            },
        ))
    }
    pub(crate) fn finish(
        &mut self,
        generation: u64,
        result: Result<PiDirectoryState, AggregationApiError>,
    ) -> bool {
        if self.pending != Some(generation) {
            return false;
        }
        self.pending = None;
        match result {
            Ok(state) => {
                self.snapshot = Some(state);
                self.removing = None;
                self.error = None;
            }
            Err(error) => {
                self.snapshot = None;
                self.removing = None;
                self.error = Some(feedback(&error));
            }
        }
        true
    }
}
impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PiDirectoryEvent")
    }
}
impl std::fmt::Debug for Form {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PiDirectoryForm")
    }
}
fn feedback(error: &AggregationApiError) -> &'static str {
    match error {
        AggregationApiError::Http { status: 401 | 403, .. } => "无权读取或修改此项目的 Pi 目录",
        AggregationApiError::Http { status: 409, .. } => {
            "目录已变化或存在待恢复事务，请刷新后重新确认"
        }
        AggregationApiError::Http { status: 404, .. } => "条目不存在，请刷新目录",
        AggregationApiError::Http { status: 422, body }
            if matches!(
                body.pointer("/error/code").and_then(serde_json::Value::as_str),
                Some("unsupported_app" | "unsupported_target")
            ) =>
        {
            "当前执行主机不支持 Pi 模型目录"
        }
        AggregationApiError::Http { status: 400 | 422, .. } => {
            "目标不支持或模型声明无效，请检查后刷新"
        }
        AggregationApiError::Http { status: 502..=504, .. } => {
            "执行主机不可用或执行失败，请刷新确认实际状态"
        }
        _ => "Pi 目录请求失败，请刷新确认实际状态",
    }
}
pub(crate) fn message(event: Event) -> Message {
    list::message(ListMessage::PiDirectory(event))
}
pub(crate) async fn execute(
    client: GatewayClient,
    query: AggregationQuery,
    write: Option<(String, PiDirectoryWrite)>,
) -> Result<PiDirectoryState, AggregationApiError> {
    match write {
        Some((id, body)) => client.pi_directory_entry(&id, &body).await,
        None => client.pi_directory_state(&query).await,
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
