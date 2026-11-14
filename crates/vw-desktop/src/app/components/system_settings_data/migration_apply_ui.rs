//! Conflict confirmation and task transport; execution and rollback remain server owned.
use super::migration_preview_ui;
use crate::app::{App, Message, message::SettingsMessage};
use crate::app::components::system_settings_common::*;
use iced::{Element, Task};
use iced::widget::{button, column, row, text};
use vw_gateway_client::vw_api_types::agent_apps::data_management::*;

#[derive(Clone, Default)]
pub(crate) struct State {
    pub(super) preview: Option<DataPreviewId>,
    decisions: Vec<DataConflictResolution>,
    pub(super) request: Option<ExecuteDataImportRequest>,
    progress: Option<DataOperationProgress>,
    context: Option<(String, u64)>,
    busy: bool,
    generation: u64,
}
#[derive(Debug, Clone)]
pub(crate) enum Event {
    Decide(DataPreviewId, String, DataConflictDecision),
    Execute(DataImportPreview), Refresh, Cancel, Retry, Reset,
    Loaded(u64, Result<DataOperationProgress, ()>),
}
fn message(event: Event) -> Message {
    Message::Settings(SettingsMessage::MigrationPreview(migration_preview_ui::Event::Apply(event)))
}
impl State {
    fn resolved(&self, preview: &DataImportPreview) -> bool {
        preview.conflicts.iter().all(|c| self.decisions.iter().any(|r|
            r.conflict_id == c.conflict_id && c.allowed_decisions.contains(&r.decision)))
    }
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let Some(target) = migration_preview_ui::target(app) else { return Task::none(); };
    let Ok(client) = crate::app::config::gateway_client() else { return Task::none(); };
    let context = (client.endpoint().base_url(), app.apps_catalog.generation);
    let token = app.apps_catalog.token.clone();
    let state = &mut app.apps_catalog.migration_preview.apply;
    if let Event::Loaded(generation, result) = event {
        if generation != state.generation || state.context.as_ref() != Some(&context) { return Task::none(); }
        state.busy = false;
        match result {
            Ok(progress) if progress.target == target && progress.operation == DataOperation::Import
                && state.progress.as_ref().is_none_or(|old| old.operation_id == progress.operation_id) => state.progress = Some(progress),
            _ => return app.show_error_toast("迁入请求未确认；请重试查询，提交重试会复用原幂等键"),
        }
        return Task::none();
    }
    if state.context.as_ref().is_some_and(|old| old != &context)
        || state.request.as_ref().is_some_and(|r| r.target != target) {
        *state = State { generation: state.generation.wrapping_add(1), ..State::default() };
    }
    if state.busy { return Task::none(); }
    match event {
        Event::Decide(preview, conflict_id, decision) => {
            if state.request.is_some() { return Task::none(); }
            if state.preview.as_ref() != Some(&preview) { state.decisions.clear(); state.preview = Some(preview); }
            state.decisions.retain(|r| r.conflict_id != conflict_id);
            state.decisions.push(DataConflictResolution { conflict_id, decision });
            Task::none()
        }
        Event::Execute(preview) => {
            if preview.target != target || (state.preview.as_ref() != Some(&preview.preview_id) && !preview.conflicts.is_empty())
                || !state.resolved(&preview) || state.progress.is_some() { return Task::none(); }
            if state.preview.as_ref() != Some(&preview.preview_id) {
                state.decisions.clear();
                state.preview = Some(preview.preview_id.clone());
            }
            if let Some(request) = &state.request {
                if request.preview_id != preview.preview_id || request.expected_revision != preview.revision { return Task::none(); }
            } else {
                state.request = Some(ExecuteDataImportRequest { target, preview_id: preview.preview_id,
                    expected_revision: preview.revision, conflicts: state.decisions.clone(),
                    idempotency_key: format!("desktop-import-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()), confirmed: true });
            }
            let request = state.request.clone().expect("request prepared");
            state.busy = true; state.context = Some(context); state.generation = state.generation.wrapping_add(1);
            let generation = state.generation;
            Task::perform(async move { client.data_migration_execute(&request, Some(&token)).await.map_err(|_| ()) }, move |r| message(Event::Loaded(generation, r)))
        }
        action @ (Event::Refresh | Event::Cancel | Event::Retry) => {
            let Some(progress) = &state.progress else { return Task::none(); };
            let input = GetDataOperationRequest { target, operation_id: progress.operation_id.clone() };
            state.busy = true; state.context = Some(context); state.generation = state.generation.wrapping_add(1);
            let generation = state.generation;
            Task::perform(async move {
                match action {
                    Event::Cancel => client.data_migration_cancel(&CancelDataOperationRequest { target: input.target, operation_id: input.operation_id }, Some(&token)).await,
                    Event::Retry => client.data_migration_retry_projections(&input, Some(&token)).await,
                    _ => client.data_migration_operation(&input, Some(&token)).await,
                }.map_err(|_| ())
            }, move |r| message(Event::Loaded(generation, r)))
        }
        Event::Reset => {
            if state.progress.as_ref().is_some_and(|p| p.result.is_some()) {
                *state = State { generation: state.generation.wrapping_add(1), ..State::default() };
            }
            Task::none()
        }
        Event::Loaded(..) => unreachable!(),
    }
}
pub(crate) fn view<'a>(app: &'a App, preview: &'a DataImportPreview) -> Element<'a, Message> {
    let state = &app.apps_catalog.migration_preview.apply;
    let locked = state.busy || state.request.is_some();
    let mut body = column![text("逐项确认冲突后提交；取消请求不代表已经回滚。")].spacing(10);
    for (index, conflict) in preview.conflicts.iter().enumerate() {
        let mut choices = row![text(format!("冲突 {} · {:?}", index + 1, conflict.category))].spacing(8);
        for decision in &conflict.allowed_decisions {
            let selected = state.preview.as_ref() == Some(&preview.preview_id) && state.decisions.iter().any(|r| r.conflict_id == conflict.conflict_id && r.decision == *decision);
            let label = match decision { DataConflictDecision::Skip => "跳过", DataConflictDecision::KeepBoth => "保留两者", DataConflictDecision::Replace => "替换" };
            choices = choices.push(button(text(format!("{}{}", if selected { "✓ " } else { "" }, label))).padding([8, 14]).style(rounded_action_btn_style)
                .on_press_maybe((!locked).then(|| message(Event::Decide(preview.preview_id.clone(), conflict.conflict_id.clone(), *decision)))));
        }
        body = body.push(choices);
    }
    if let Some(progress) = &state.progress {
        body = body.push(text(format!("阶段 {:?} · 已处理 {} / {:?}\n已提交：{} · 取消请求：{} · 回滚：{:?}", progress.phase, progress.completed_items, progress.total_items, progress.committed, progress.cancel_requested, progress.rollback)));
        let result = match &progress.result { None => "迁入进行中", Some(DataOperationResult::Succeeded) => "迁入成功", Some(DataOperationResult::Partial { .. }) => "部分完成，请检查投影结果", Some(DataOperationResult::Failed { .. }) => "迁入失败，请检查回滚状态", Some(DataOperationResult::Cancelled) => "迁入已取消，请检查回滚状态" };
        body = body.push(text(result));
        if progress.result.is_some() {
            body = body.push(button(text("结束查看，准备新的迁入")).padding([8, 14]).style(rounded_action_btn_style).on_press_maybe((!state.busy).then(|| message(Event::Reset))));
        }
        let retry = progress.committed && matches!(&progress.result, Some(DataOperationResult::Partial { errors }) if errors.iter().any(|e| e.code == DataManagementErrorCode::ProjectionFailed));
        for (label, event, enabled) in [("刷新进度", Event::Refresh, true), ("请求取消", Event::Cancel, progress.result.is_none() && !progress.cancel_requested), ("重试失败投影", Event::Retry, retry)] {
            body = body.push(button(text(label)).padding([8, 14]).style(rounded_action_btn_style).on_press_maybe((enabled && !state.busy).then(|| message(event))));
        }
    } else {
        body = body.push(button(text(if state.busy { "提交中…" } else { "确认迁入 / 重试提交" })).padding([8, 14]).style(primary_action_btn_style)
            .on_press_maybe((!state.busy && state.resolved(preview)).then(|| message(Event::Execute(preview.clone())))));
    }
    body.into()
}
