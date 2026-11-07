//! Desktop interaction state; plans, preflight and execution belong to the gateway.
use vw_gateway_client::vw_api_types::{agent_apps::*, task::TaskStatus};

#[derive(Clone, Debug, Default)]
pub(crate) struct LifecycleState {
    pub target_label: String,
    pub request: Option<AppActionPreviewRequest>,
    pub preview: Option<AppActionPreview>,
    pub operation: Option<AppOperationStatus>,
    pub busy: bool,
    pub uncertain: bool,
    pub error: Option<String>,
    pub generation: u64,
}
impl LifecycleState {
    pub fn invalidate(&mut self) {
        let generation = self.generation.wrapping_add(1);
        *self = Self { generation, ..Default::default() };
    }
    pub fn accept_status(&mut self, status: AppOperationStatus) -> bool {
        if let Some(current) = &self.operation {
            if current.operation_id != status.operation_id
                || current.phase == AppOperationPhase::Terminal
            {
                return false;
            }
        }
        self.operation = Some(status);
        true
    }
    pub fn active(&self) -> bool {
        self.busy
            || self.uncertain
            || self
                .operation
                .as_ref()
                .is_some_and(|s| matches!(s.status, TaskStatus::Queued | TaskStatus::Running))
    }
}

use super::message;
use crate::app::components::system_settings_common::*;
use crate::app::{
    Message,
    message::settings::{AppsCatalogMessage, apps_lifecycle::LifecycleMessage},
};
use iced::{
    Alignment, Element, Length,
    widget::{Space, button, column, container, row, text},
};
fn event(value: LifecycleMessage) -> Message {
    message(AppsCatalogMessage::Lifecycle(value))
}
pub(crate) fn action_button<'a>(
    label: &'a str,
    request: AppActionPreviewRequest,
    enabled: bool,
) -> Element<'a, Message> {
    button(text(label))
        .padding([8, 16])
        .style(rounded_action_btn_style)
        .on_press_maybe(enabled.then_some(event(LifecycleMessage::Preview(request))))
        .into()
}
pub(crate) fn progress(state: &LifecycleState) -> Element<'_, Message> {
    let mut content = column![].spacing(8);
    if let Some(request) = &state.request {
        content = content.push(
            text(format!(
                "应用：{:?} · {}",
                request.app_id,
                match request.action {
                    AppLifecycleAction::Install => "安装",
                    AppLifecycleAction::Update => "升级",
                    AppLifecycleAction::Uninstall => "卸载",
                }
            ))
            .size(13),
        );
    }
    if !state.target_label.is_empty() {
        content = content.push(text(&state.target_label).size(13));
    }
    if let Some(operation) = &state.operation {
        content = content.push(
            text(format!(
                "操作 {}\n{:?} · {:?}",
                operation.operation_id.0, operation.phase, operation.status
            ))
            .size(13),
        );
        if operation.phase != AppOperationPhase::Terminal {
            content = content.push(
                row![
                    button(text("刷新操作 / 恢复连接"))
                        .padding([8, 16])
                        .style(rounded_action_btn_style)
                        .on_press(event(LifecycleMessage::Refresh)),
                    button(text("取消操作"))
                        .padding([8, 16])
                        .style(rounded_action_btn_style)
                        .on_press(event(LifecycleMessage::Cancel)),
                ]
                .spacing(10),
            );
        }
    } else if state.busy {
        content = content.push(text("正在预检或提交，请稍候…"));
    }
    if let Some(error) = &state.error {
        content = content.push(settings_error_banner(error.clone()));
    }
    content.into()
}
pub(crate) fn overlay<'a>(
    state: &'a LifecycleState,
    host: &'a str,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let Some(preview) = &state.preview else {
        return base;
    };
    let close = event(LifecycleMessage::Close);
    let (title, guidance, confirm_label) = match preview.request.action {
        AppLifecycleAction::Install => {
            ("确认安装", "确认后由网关重新检查目标与 Node 依赖；排队不表示安装成功。", "确认提交")
        }
        AppLifecycleAction::Update => {
            ("确认升级", "确认后由网关重新检查目标与 Node 依赖；排队不表示升级成功。", "确认提交")
        }
        AppLifecycleAction::Uninstall => {
            ("确认卸载所选实例", super::uninstall::guidance(preview.request.app_id), "确认卸载")
        }
    };
    let uninstalling = preview.request.action == AppLifecycleAction::Uninstall;
    let header = settings_modal_header(
        settings_page_heading(title, "请核对执行主机和预检目标。"),
        settings_close_button(close.clone()),
    );
    let mut body = column![
        text(format!("执行主机：{host}")).size(14),
        text(format!("来源：{:?}", preview.source)).size(13).style(settings_muted_text_style),
        text(&state.target_label).size(14).wrapping(iced::widget::text::Wrapping::WordOrGlyph),
        settings_divider(),
        text(&preview.summary).size(14).wrapping(iced::widget::text::Wrapping::WordOrGlyph),
        text(guidance).size(13).style(settings_muted_text_style),
    ]
    .spacing(12);
    if uninstalling {
        body = body.push(
            text("计划或路径变化时拒绝执行；请重新预检并确认，不会自动重试删除。")
                .size(13)
                .style(settings_muted_text_style),
        );
    }
    let footer = row![
        Space::new().width(Length::Fill),
        button(text("取消").size(SETTINGS_CONTROL_TEXT_SIZE))
            .padding([8, 16])
            .style(rounded_action_btn_style)
            .on_press(close.clone()),
        button(
            text(if state.busy { "处理中…" } else { confirm_label })
                .size(SETTINGS_CONTROL_TEXT_SIZE)
        )
        .padding([8, 16])
        .style(if uninstalling { danger_action_btn_style } else { primary_action_btn_style })
        .on_press_maybe(
            (!state.busy
                && !state.uncertain
                && state.request.as_ref() == Some(&preview.request)
                && preview.source != AppInstallationSource::Unknown)
                .then_some(event(LifecycleMessage::Confirm)),
        ),
    ]
    .align_y(Alignment::Center)
    .spacing(10);
    let card = settings_modal_layout(header, body, Some(footer.into()))
        .width(Length::Fill)
        .max_width(520.0)
        .max_height(484.0);
    settings_modal_overlay(Some(base), close, container(card).padding(12).center(Length::Fill))
}
