//! Backend-owned budgets and switches, scoped to the existing memory page.
use super::{
    hermes_memory::{self, Scope},
    system_settings_common::*,
};
use crate::app::{App, Message};
use iced::{
    Element, Task,
    widget::{button, column, text},
};
use vw_gateway_client::{GatewayClient, HermesMemoryScope, vw_api_types::hermes_memory::*};

#[derive(Clone, Default)]
pub(crate) struct State {
    pub limits: Option<HermesMemoryLimits>,
    pub model: Option<HermesModelSnapshot>,
    pub model_loaded: bool,
    pub busy: bool,
    pub error: Option<String>,
}
#[derive(Clone)]
pub enum Event {
    Refresh,
    Toggle(usize, bool),
}
#[derive(Clone)]
pub enum Reply {
    Loaded(Result<HermesMemoryLimits, String>, Result<Option<HermesModelSnapshot>, String>),
    Toggled(Result<(HermesWriteOutcome, HermesMemoryLimits), String>),
}
fn message(event: Event) -> Message {
    hermes_memory::message(hermes_memory::Event::Limits(event))
}

pub(crate) fn budget_text(content: &str, limit: u64) -> String {
    let used = content.encode_utf16().count() as u64;
    if used > limit {
        format!("超出预算 {} / {limit}（UTF-16 单元）；原文保留", used - limit)
    } else {
        format!("剩余 {} / {limit}（UTF-16 单元）", limit - used)
    }
}
pub(crate) fn view<'a>(state: &'a State, index: usize, content: &str) -> Element<'a, Message> {
    let mut panel = column![].spacing(8);
    if let Some(limits) = &state.limits {
        let (limit, enabled) = if index == 0 {
            (limits.memory, limits.memory_enabled)
        } else {
            (limits.user, limits.user_enabled)
        };
        let mut toggle = settings_toggle(enabled).label("启用当前记忆");
        if !state.busy {
            toggle = toggle.on_toggle(move |enabled| message(Event::Toggle(index, enabled)));
        }
        panel = panel.push(toggle).push(text(budget_text(content, limit)));
        if !enabled {
            panel = panel.push(text("已禁用：Hermes 将跳过此记忆，仍可编辑和保存文件。"));
        }
    } else {
        panel = panel.push(text("预算与开关尚未读取；读取成功后可切换。"));
    }
    if let Some(error) = &state.error {
        panel = panel.push(text(error));
    }
    if state.model_loaded {
        let info = match &state.model {
            Some(model) => format!(
                "模型（只读）：{}；提供商：{}；上下文：{}；最大输出：{}。修改请使用供应商设置。",
                model.default.as_deref().unwrap_or("未配置"),
                model.provider.as_deref().unwrap_or("未配置"),
                model.context_length.map_or("未配置".into(), |n| n.to_string()),
                model.max_tokens.map_or("未配置".into(), |n| n.to_string())
            ),
            None => "模型（只读）：未配置；修改请使用供应商设置。".into(),
        };
        panel = panel.push(text(info));
    } else {
        panel = panel.push(text("只读模型信息尚未读取"));
    }
    panel
        .push(
            button(text(if state.busy { "正在同步设置" } else { "刷新预算与模型" }))
                .padding([8, 12])
                .style(rounded_action_btn_style)
                .on_press_maybe((!state.busy).then(|| message(Event::Refresh))),
        )
        .into()
}
pub(crate) fn request(
    state: &mut State,
    action: Event,
    client: GatewayClient,
    query: HermesMemoryScope,
    token: String,
    scope: Scope,
) -> Task<Message> {
    if state.busy {
        return Task::none();
    }
    if let Event::Toggle(index, _) = &action {
        if *index > 1 || state.limits.is_none() {
            return Task::none();
        }
    }
    state.busy = true;
    state.error = None;
    Task::perform(
        async move {
            let reply = match action {
                Event::Refresh => Reply::Loaded(
                    client.hermes_memory_limits(&query, &token).await.map_err(|e| e.to_string()),
                    client.hermes_model(&query, &token).await.map_err(|e| e.to_string()),
                ),
                Event::Toggle(index, enabled) => {
                    let result = async {
                        let outcome = client
                            .set_hermes_memory_enabled(
                                &query,
                                hermes_memory::kind(index),
                                enabled,
                                &token,
                            )
                            .await
                            .map_err(|e| format!("开关保存失败：{e}"))?;
                        let limits = client
                            .hermes_memory_limits(&query, &token)
                            .await
                            .map_err(|e| format!("开关已保存，但刷新失败，请重试：{e}"))?;
                        Ok((outcome, limits))
                    }
                    .await;
                    Reply::Toggled(result)
                }
            };
            hermes_memory::message(hermes_memory::Event::Metadata(scope, reply))
        },
        |message| message,
    )
}
pub(crate) fn apply(app: &mut App, reply: Reply) -> Task<Message> {
    let state = &mut app.apps_catalog.hermes_memory.limits;
    state.busy = false;
    match reply {
        Reply::Loaded(limits, model) => {
            match limits {
                Ok(value) => state.limits = Some(value),
                Err(error) => state.error = Some(format!("预算读取失败：{error}")),
            }
            match model {
                Ok(value) => {
                    state.model = value;
                    state.model_loaded = true;
                }
                Err(error) => {
                    let prefix = state.error.take().map_or(String::new(), |e| format!("{e}；"));
                    state.error = Some(format!("{prefix}模型读取失败：{error}"));
                }
            }
        }
        Reply::Toggled(Ok((outcome, limits))) => {
            state.limits = Some(limits);
            return app.show_success_toast(match outcome.backup_path {
                Some(path) => format!("记忆开关已保存并刷新；备份：{path}"),
                None => "记忆开关已保存并刷新".into(),
            });
        }
        Reply::Toggled(Err(error)) => {
            state.error = Some(error.clone());
            return app.show_error_toast(error);
        }
    }
    Task::none()
}
