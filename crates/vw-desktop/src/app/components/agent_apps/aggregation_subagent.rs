//! Dedicated subagent modal using the shared themed controls and scroll layout.
use crate::app::{
    Message,
    components::system_settings_common::*,
    message::settings::aggregation_subagent::{Event, Form, message},
};
use iced::{
    Element, Length,
    widget::{button, column, container, row, text},
};
use vw_gateway_client::vw_api_types::{
    ReasoningEffortDto, agent_apps::aggregation::AggregationNotice,
};
#[derive(Clone, PartialEq)]
struct ModelOption {
    id: Option<String>,
    label: String,
}
impl std::fmt::Display for ModelOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}
#[derive(Clone, PartialEq)]
struct EffortOption(Option<ReasoningEffortDto>);
impl std::fmt::Display for EffortOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self.0 {
            None => "模型默认",
            Some(ReasoningEffortDto::None) => "none",
            Some(ReasoningEffortDto::Minimal) => "minimal",
            Some(ReasoningEffortDto::Low) => "low",
            Some(ReasoningEffortDto::Medium) => "medium",
            Some(ReasoningEffortDto::High) => "high",
            Some(ReasoningEffortDto::Xhigh) => "xhigh",
            Some(ReasoningEffortDto::Max) => "max",
        };
        f.write_str(label)
    }
}
pub(crate) fn overlay<'a>(form: &'a Form, base: Element<'a, Message>) -> Element<'a, Message> {
    let ready = form.pending.is_none();
    let editable = ready && form.snapshot.is_some();
    let action = |label: &'static str, event, enabled: bool| {
        button(text(label))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .on_press_maybe(enabled.then_some(message(event)))
    };
    let mut body = column![
        text("选择已发布的非默认成员模型。保存后需要新建 Codex 会话；已有会话保持原设置。")
            .style(settings_muted_text_style)
    ]
    .spacing(12)
    .width(Length::Fill);
    if let Some(error) = form.error {
        body = body.push(settings_error_banner(error));
    }
    if let Some(state) = &form.snapshot {
        let mut options = vec![ModelOption { id: None, label: "跟随主模型".into() }];
        options.extend(state.choices.iter().map(|c| ModelOption {
            id: Some(c.model_id.clone()),
            label: format!("{} · {}", c.label, c.model_id),
        }));
        if form.unavailable() {
            options.push(ModelOption {
                id: form.draft.subagent_model_id.clone(),
                label: format!(
                    "已移除：{}",
                    form.draft.subagent_model_id.as_deref().unwrap_or_default()
                ),
            });
            body = body.push(settings_error_banner("所选模型已不在发布目录中。保留选择意图；实际覆写已撤下，跟随主模型。可选择新模型或清空。"));
        }
        let selected = options.iter().find(|c| c.id == form.draft.subagent_model_id).cloned();
        body = body.push(settings_field_label("子代理模型")).push(
            settings_select(options, selected, |v| message(Event::Model(v.id)))
                .width(Length::Fill)
                .disabled(!editable),
        );
        let choice = form.selected();
        let mut efforts = vec![EffortOption(None)];
        if let Some(c) = choice {
            efforts.extend(c.efforts.iter().map(|e| EffortOption(Some(*e))));
        }
        let selected = EffortOption(
            form.draft.subagent_effort.filter(|e| choice.is_some_and(|c| c.efforts.contains(e))),
        );
        body = body.push(settings_field_label("推理强度")).push(
            settings_select(efforts, Some(selected), |v| message(Event::Effort(v.0)))
                .width(Length::Fill)
                .disabled(!editable || choice.is_none()),
        );
        if state.choices.is_empty() {
            body = body.push(text("暂无可选的已发布成员模型。"));
        }
        let effective_model = state.effective.subagent_model_id.as_deref().unwrap_or("跟随主模型");
        body = body.push(settings_panel(
            column![
                settings_field_label("当前生效设置"),
                text(effective_model),
                text(format!("推理强度：{}", EffortOption(state.effective.subagent_effort))),
                text("此处显示执行主机返回的已保存状态；草稿需保存后生效。").size(13)
            ]
            .spacing(6),
        ));
        if state.notices.contains(&AggregationNotice::MultiAgentV2Forced) {
            body = body.push(settings_error_banner(
                "Codex 强制启用了 multi_agent_v2，经典工具设置可能不生效。",
            ));
        }
        if state.notices.contains(&AggregationNotice::EncryptedTaskCrossProvider)
            || (!form.draft.classic_tools && choice.is_some())
        {
            body = body.push(settings_error_banner(
                "新版工具可能携带加密任务，跨供应商子代理可能无法使用。",
            ));
        }
        if state.notices.contains(&AggregationNotice::RestartDesktop)
            || state.notices.contains(&AggregationNotice::RestartEditor)
        {
            body = body.push(text("执行主机提醒：请完全退出并重开桌面或编辑器客户端。"));
        }
    } else {
        body = body.push(text(if ready {
            "请刷新已发布目录后再编辑。"
        } else {
            "正在读取或保存执行主机设置…"
        }));
    }
    if form.saved {
        body = body.push(text("已保存。请新建 Codex 会话使用新设置。"));
    }
    let close = if ready { message(Event::Close) } else { Message::None };
    let header = settings_modal_header(
        text("Codex 子代理设置").size(20),
        settings_close_button(close.clone()),
    );
    let footer = row![
        action("刷新并重载", Event::Refresh, ready),
        action("取消", Event::Close, ready),
        button(text("保存"))
            .padding([8, 16])
            .style(primary_action_btn_style)
            .on_press_maybe(editable.then_some(message(Event::Save)))
    ]
    .spacing(10);
    let card = settings_modal_layout_with_scroll_id(
        header,
        body,
        Some(footer.into()),
        Some("codex-subagent-form"),
    )
    .width(760)
    .height(650);
    settings_modal_overlay(Some(base), close, container(card).padding(12))
}
