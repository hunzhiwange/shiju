use super::list::{self, ListMessage};
use crate::app::{
    App, Message,
    state::app_providers::{
        list::Scope,
        preview::{Display, Form, SCROLL_ID, Save, SaveOutcome, available},
    },
};
use iced::{Task, widget::text_editor::Action};
use vw_gateway_client::{
    AgentAppsReadError, GatewayClient,
    vw_api_types::{
        agent_apps::AppId,
        app_providers::{
            editor_merge::{EditorChoice, EditorEdit, EditorOpen, EditorView},
            operations::{
                ProviderEditorOutcome, ProviderOperationAction, ProviderOperationProgress,
                ProviderOperationRequest,
            },
            preview::{ProviderPreviewRequest, SavedProviderPreviewRequest},
        },
        task::TaskStatus,
    },
};

#[derive(Clone)]
pub enum Event {
    Open(String),
    Close,
    Refresh,
    Reload,
    Edit(Action),
    Save(EditorChoice),
    CancelSave,
    Check,
    Loaded {
        scope: Scope,
        generation: u64,
        result: Loaded,
    },
    Progress {
        scope: Scope,
        generation: u64,
        result: Result<ProviderOperationProgress, AgentAppsReadError>,
    },
}

#[derive(Clone)]
pub struct Loaded {
    view: Result<Display, String>,
    editor: Option<EditorView>,
    warning: Option<String>,
}

impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ProviderPreviewEvent([redacted])")
    }
}

pub(crate) fn message(event: Event) -> Message {
    list::message(ListMessage::Preview(event))
}

pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let state = &app.app_providers;
    let Some(scope) = state.scope.clone() else { return Task::none() };
    if let Event::Open(id) = event {
        if !available(scope.app)
            || state.loading
            || state.form.is_some()
            || state.editor.is_some()
            || state.snippets.is_some()
            || state.universal.is_some()
            || state.icons.is_some()
            || state.endpoints.is_some()
            || state.desktop.is_some()
            || state.preview.is_some()
        {
            return Task::none();
        }
        let Some(provider) = state
            .snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.catalog.rows.iter().find(|row| row.provider.id == id))
        else {
            return Task::none();
        };
        app.app_providers.preview = Some(Form::new(id, provider.provider.name.clone()));
        return update(app, Event::Refresh);
    }
    if matches!(event, Event::Close) {
        let state = &mut app.app_providers;
        if state.preview.as_ref().is_some_and(|form| form.save.is_none()) {
            state.preview = None;
            state.generation = state.generation.wrapping_add(1);
        }
        return Task::none();
    }

    let Ok((client, now)) = list::current(app, scope.app) else { return Task::none() };
    if now != scope {
        app.app_providers.preview = None;
        return Task::none();
    }
    let token = app.apps_catalog.token.clone();
    let state = &mut app.app_providers;
    let Some(form) = &mut state.preview else { return Task::none() };
    let generation = state.generation;

    match event {
        Event::Refresh | Event::Reload if form.save.is_none() => {
            if matches!(event, Event::Refresh) {
                form.notice = None;
            }
            form.view = None;
            form.confirmation = Default::default();
            state.generation = generation.wrapping_add(1);
            let id = form.id.clone();
            let completed_scope = scope.clone();
            let generation = state.generation;
            let (task, handle) = Task::perform(load(client, scope, id, token), move |result| {
                message(Event::Loaded { scope: completed_scope.clone(), generation, result })
            })
            .abortable();
            form.reading = Some(handle.abort_on_drop());
            return Task::batch([task, scroll_top()]);
        }
        Event::Loaded { scope: completed, generation: received, result }
            if completed == scope && received == generation && form.reading.take().is_some() =>
        {
            match result.view {
                Ok(view) => form.view = Some(view),
                Err(error) => append_notice(form, error),
            }
            if let Some(warning) = result.warning {
                append_notice(form, warning);
            }
            form.confirmation.editor = result.editor;
            return scroll_top();
        }
        Event::Edit(action) if form.save.is_none() && form.confirmation.conflict.is_none() => {
            form.draft.perform(action);
        }
        Event::Save(choice) if form.save.is_none() && form.reading.is_none() => {
            let edits: Vec<EditorEdit> = match serde_json::from_str(&form.draft.text()) {
                Ok(edits) => edits,
                Err(_) => return app.show_error_toast("请填写有效的 JSON 改动列表。"),
            };
            if edits.is_empty() {
                return app.show_error_toast("请先填写要保存的改动。");
            }
            let Some(input) = form.confirmation.request(scope.host.clone(), edits, choice) else {
                return Task::none();
            };
            let mut random = [0; 16];
            rand::fill(&mut random);
            form.save = Some(Save {
                request: ProviderOperationRequest {
                    target_host_id: scope.host.clone(),
                    request_id: hex::encode(random),
                    action: ProviderOperationAction::EditorSave {
                        app: scope.app,
                        id: form.id.clone(),
                        input,
                    },
                },
                choice,
                progress: None,
                cancel_requested: false,
                waiting: false,
            });
            form.notice = None;
            state.generation = generation.wrapping_add(1);
            return Task::batch([update(app, Event::Check), scroll_top()]);
        }
        Event::CancelSave => {
            if let Some(save) = &mut form.save {
                save.cancel_requested = true;
                return update(app, Event::Check);
            }
        }
        Event::Check => {
            if let Some(save) = form.save.as_mut().filter(|save| !save.waiting) {
                save.waiting = true;
                return check(client, scope, token, generation, save.clone());
            }
        }
        Event::Progress { scope: completed, generation: received, result }
            if completed == scope && received == generation =>
        {
            let Some(save) = &mut form.save else { return Task::none() };
            save.waiting = false;
            match result {
                Ok(progress)
                    if progress.request_id == save.request.request_id
                        && save.progress.as_ref().is_none_or(|previous| {
                            previous.operation_id == progress.operation_id
                        }) =>
                {
                    if matches!(progress.status, TaskStatus::Queued | TaskStatus::Running) {
                        save.progress = Some(progress);
                        form.notice = None;
                        return update(app, Event::Check);
                    }
                    let choice = save.choice;
                    form.save = None;
                    if progress.status == TaskStatus::Cancelled {
                        form.notice = Some("执行主机已取消保存。".into());
                        return Task::none();
                    }
                    let result = match progress.editor_outcome {
                        Some(ProviderEditorOutcome::Reviewed { result })
                            if progress.status == TaskStatus::Completed =>
                        {
                            Ok(result)
                        }
                        Some(ProviderEditorOutcome::Rejected { status }) => Err(status),
                        _ => Err(503),
                    };
                    match form.confirmation.finish(choice, result) {
                        SaveOutcome::Saved => {
                            form.draft = iced::widget::text_editor::Content::with_text("[]");
                            form.notice = Some("改动已保存。".into());
                            return Task::batch([
                                app.show_success_toast("配置改动已保存"),
                                update(app, Event::Reload),
                            ]);
                        }
                        SaveOutcome::Conflict => {
                            form.notice = Some("配置已被其他程序修改，请选择如何处理冲突。".into());
                            return scroll_top();
                        }
                        SaveOutcome::Reload => {
                            form.notice = Some(
                                "配置或确认已过期。草稿已保留，请核对新预览后重新确认。".into(),
                            );
                            return update(app, Event::Reload);
                        }
                        SaveOutcome::Rejected => {
                            form.notice = Some("执行主机拒绝保存，请检查改动和配置权限。".into());
                        }
                    }
                }
                Ok(_) => {
                    form.notice = Some("进度与本次请求不符，请重新查询。".into());
                }
                Err(error) => {
                    if matches!(
                        error,
                        AgentAppsReadError::Http {
                            status: 400 | 401 | 403 | 404 | 409 | 413 | 422 | 501,
                            ..
                        }
                    ) {
                        let started = save.progress.is_some();
                        form.save = None;
                        form.notice = Some(if started {
                            "无法继续查询保存结果，请重新读取配置。".into()
                        } else {
                            "保存请求被拒绝，请检查配置权限和改动。".into()
                        });
                        if started {
                            return update(app, Event::Reload);
                        }
                    } else {
                        form.notice = Some("暂时无法读取保存进度，请重试查询。".into());
                    }
                }
            }
        }
        _ => {}
    }
    Task::none()
}

fn scroll_top() -> Task<Message> {
    iced::widget::operation::snap_to(
        iced::widget::Id::new(SCROLL_ID),
        iced::widget::scrollable::RelativeOffset::START,
    )
    .map(|(): ()| Message::None)
}

fn append_notice(form: &mut Form, notice: String) {
    if let Some(previous) = &mut form.notice {
        previous.push('\n');
        previous.push_str(&notice);
    } else {
        form.notice = Some(notice);
    }
}

fn check(
    client: GatewayClient,
    scope: Scope,
    token: String,
    generation: u64,
    save: Save,
) -> Task<Message> {
    let completed_scope = scope.clone();
    Task::perform(
        async move {
            match save.progress {
                Some(progress) if save.cancel_requested && !progress.cancellation_requested => {
                    client
                        .provider_operation_cancel(
                            progress.operation_id.as_ref(),
                            &scope.host,
                            &token,
                        )
                        .await
                }
                Some(progress) => {
                    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                    client
                        .provider_operation_get(progress.operation_id.as_ref(), &scope.host, &token)
                        .await
                }
                None => client.provider_operation_start(&save.request, &token).await,
            }
        },
        move |result| {
            message(Event::Progress { scope: completed_scope.clone(), generation, result })
        },
    )
}

async fn load(client: GatewayClient, scope: Scope, id: String, token: String) -> Loaded {
    let preview = client
        .app_provider_preview(
            scope.app,
            &ProviderPreviewRequest {
                target_host_id: scope.host.clone(),
                directory_override: None,
                settings: serde_json::Value::Null,
                category: None,
                provider_id: Some(id.clone()),
                meta: None,
                request_options: None,
            },
            &token,
        )
        .await;
    let warning = match &preview {
        Ok(value) if value.parse_error.is_some() => value.parse_error.as_ref().map(|error| {
            format!("无法解析客户端配置（第 {} 行，第 {} 列）。", error.line, error.column)
        }),
        Ok(value) if value.unavailable.is_some() => value.unavailable.as_ref().map(|reason| {
            use vw_gateway_client::vw_api_types::app_providers::preview::PreviewUnavailable::*;
            match reason {
                UnsupportedApplication => "此应用不支持配置投影。",
                OfficialLogin => "官方登录配置无法预览。",
                Ownership => "当前执行主机的配置归属不可用。",
                PendingTransaction => "执行主机有待恢复的配置事务。",
                ManagedProvider => "托管供应商配置无法预览。",
                ModelCatalog => "模型目录配置无法预览。",
                InvalidDraft => "已保存的供应商配置无法投影。",
            }
            .into()
        }),
        Err(_) => Some("无法读取客户端配置，请检查执行主机和配置读取权限。".into()),
        _ => None,
    };
    if let Ok(preview) = preview
        && warning.is_none()
    {
        let editor = if scope.app == AppId::Claude {
            client
                .app_provider_editor_open(
                    scope.app,
                    &id,
                    &EditorOpen { target_host_id: scope.host },
                    &token,
                )
                .await
                .ok()
        } else {
            None
        };
        let warning = (scope.app == AppId::Claude && editor.is_none())
            .then(|| "无法打开可保存的编辑上下文，仍可查看脱敏差异。".into());
        return Loaded { view: Ok(Display::Native(preview)), editor, warning };
    }

    let view = client
        .app_provider_saved_preview(
            scope.app,
            &SavedProviderPreviewRequest { target_host_id: scope.host, provider_id: id },
            &token,
        )
        .await
        .map(|value| Display::Saved(value.settings))
        .map_err(|_| "也无法读取已保存的供应商配置。".into());
    Loaded { view, editor: None, warning }
}

#[cfg(test)]
#[path = "preview_tests.rs"]
mod tests;
