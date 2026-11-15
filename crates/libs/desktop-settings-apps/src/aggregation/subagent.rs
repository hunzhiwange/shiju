//! Codex settings update with explicit form and request context.
use super::{Notice, error, info};
use iced::Task;
use vw_gateway_client::GatewayClient;
use gateway_app_settings::AgentAppsSettingsError;
use vw_api_app_aggregation::*;
pub use desktop_apps_ui::aggregation::subagent::{Context, Event, Form};
pub(super) async fn execute(
    client: GatewayClient,
    context: Context,
    write: Option<CodexSettingsRequest>,
) -> Result<CodexAggregationSettings, AgentAppsSettingsError> {
    let client = gateway_aggregation_codex::GatewayAggregationCodexClient::new(client.transport().clone());
    match write {
        Some(body) => client.aggregation_codex_settings_save(&body).await,
        None => {
            client
                .aggregation_codex_settings(&AggregationQuery {
                    target_host_id: context.scope.host,
                    project_id: context.project,
                })
                .await
        }
    }
}
pub fn update(
    slot: &mut Option<Form>, generation_counter: &mut u64, blocked: bool,
    client: GatewayClient, context: Context, event: Event, notice: &mut Option<Notice>,
) -> Task<Event> {
    if matches!(event, Event::Open) {
        if blocked || slot.is_some() {
            return Task::none();
        }
        *slot = Some(Form { context: Some(context.clone()), ..Default::default() });
        return update(slot, generation_counter, blocked, client, context, Event::Refresh, notice);
    }
    let Some(form) = slot else { return Task::none() };
    if form.context.as_ref() != Some(&context) {
        *slot = None;
        return Task::none();
    }
    if let Event::Finished { context: old, generation, saved, result } = event {
        if old != context || !form.finish(generation, saved, result) {
            return Task::none();
        }
        return if let Some(e) = form.error.clone() {
            error(notice, e)
        } else if saved {
            info(notice, "Codex 设置已保存，请新建 Codex 会话后使用。")
        } else {
            Task::none()
        };
    }
    if form.pending.is_some() {
        return Task::none();
    }
    let write = match event {
        Event::Close => {
            *slot = None;
            return Task::none();
        }
        Event::Model(id) if form.snapshot.is_some() => {
            form.select_model(id);
            return Task::none();
        }
        Event::Effort(e) if form.snapshot.is_some() => {
            form.select_effort(e);
            return Task::none();
        }
        Event::Review(id) => {
            form.select_review(id);
            return Task::none();
        }
        Event::Classic(enabled) => {
            form.select_classic(enabled);
            return Task::none();
        }
        Event::Refresh => {
            form.snapshot = None;
            None
        }
        Event::Save => match form.write(&context) {
            Some(w) => Some(w),
            None => return error(notice, "请刷新配置后再保存"),
        },
        _ => return Task::none(),
    };
    let saved = write.is_some();
    *generation_counter = generation_counter.wrapping_add(1);
    let generation = *generation_counter;
    form.pending = Some(generation);
    form.error = None;
    form.saved = false;
    Task::perform(execute(client, context.clone(), write), move |result| {
        Event::Finished { context: context.clone(), generation, saved, result }
    })
}
