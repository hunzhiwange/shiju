//! 员工设置中的独立 Worker 网关资源管理面板。

use crate::app::components::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style, settings_error_banner,
    settings_muted_text_style, settings_panel_style, settings_section_card,
    settings_segment_button_style, settings_success_banner, settings_text_input_style,
    settings_value_badge,
};
use crate::app::message::settings::{
    SettingsMessage, StaffMessage, StaffResourceMessage, StaffWorkerDraftField,
};
use crate::app::state::StaffWorkerDraft;
use crate::app::{App, Message};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{button, column, container, pick_list, row, scrollable, text, text_input};
use iced::{Alignment, Element, Length, Theme};
use vw_gateway_client::vw_api_types::agent_scheduler::{
    AgentCapabilityDto, AgentCapabilityKindDto,
};

const CAPABILITY_KINDS: &[(&str, &str)] = &[
    ("Skill", "skill"),
    ("Remote Skill", "remote_skill"),
    ("MCP Server", "mcp_server"),
    ("Model", "model"),
    ("Runtime", "runtime"),
    ("Tool", "tool"),
];

pub(super) fn view(app: &App) -> Element<'_, Message> {
    let state = &app.staff_settings.workers;
    let has_project = app.project_path.as_deref().is_some_and(|path| !path.trim().is_empty());

    let refresh = action_button(
        if state.loading { "刷新中" } else { "刷新" },
        !state.loading && has_project,
        StaffResourceMessage::Refresh,
        true,
    );
    let add = action_button(
        "新增 Worker",
        !state.loading && !state.saving && has_project,
        StaffResourceMessage::WorkerNewRequested,
        false,
    );

    let mut panel = column![
        row![
            column![
                text("Workers").size(16),
                text("经 GatewayClient 管理项目 Worker 注册配置与网关快照。")
                    .size(12)
                    .style(settings_muted_text_style),
            ]
            .spacing(4)
            .width(Length::Fill),
            row![refresh, add].spacing(10).wrap(),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
        row![
            settings_value_badge(format!("{} Workers", state.workers.len())),
            settings_value_badge(if has_project { "项目已选择" } else { "未选择项目" }),
            settings_value_badge(if state.loading { "同步中" } else { "已就绪" }),
            settings_value_badge(if state.saving { "保存中" } else { "可编辑" }),
        ]
        .spacing(10)
        .wrap(),
    ]
    .spacing(14);

    if let Some(message) = state.success.as_deref() {
        panel = panel.push(settings_success_banner(message));
    }
    if let Some(message) = state.error.as_deref() {
        panel = panel.push(settings_error_banner(message));
    }

    panel = panel.push(
        row![worker_list(app), worker_detail(app, has_project)].spacing(20).height(Length::Fill),
    );

    container(panel.height(Length::Fill))
        .padding([18, 20])
        .width(Length::Fill)
        .height(Length::Fill)
        .style(settings_panel_style)
        .into()
}

fn worker_list(app: &App) -> Element<'_, Message> {
    let state = &app.staff_settings.workers;
    let rows: Element<'_, Message> = if state.loading && state.workers.is_empty() {
        centered_state("正在加载 Worker…")
    } else if state.workers.is_empty() {
        centered_state("暂无 Worker；可新建项目级注册配置。")
    } else {
        column(
            state
                .workers
                .iter()
                .map(|worker| {
                    let selected = state.selected_worker_id.as_deref() == Some(worker.id.as_str());
                    let worker_id = worker.id.clone();
                    button(
                        column![
                            text(worker.id.clone()).size(13),
                            text(format!("{} · {:?}", worker.model, worker.status))
                                .size(11)
                                .style(settings_muted_text_style),
                        ]
                        .spacing(4)
                        .width(Length::Fill),
                    )
                    .padding([10, 12])
                    .width(Length::Fill)
                    .on_press(resource_message(StaffResourceMessage::WorkerSelected(worker_id)))
                    .style(move |theme: &Theme, status| {
                        settings_segment_button_style(theme, status, selected)
                    })
                    .into()
                })
                .collect::<Vec<_>>(),
        )
        .spacing(8)
        .into()
    };

    container(
        column![
            text("Worker 列表").size(14),
            scrollable(container(rows).padding(iced::Padding::default().right(10.0)))
                .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
                .height(Length::Fill),
        ]
        .spacing(12)
        .height(Length::Fill),
    )
    .padding([16, 16])
    .width(Length::Fixed(286.0))
    .height(Length::Fill)
    .style(settings_panel_style)
    .into()
}

fn worker_detail(app: &App, has_project: bool) -> Element<'_, Message> {
    let state = &app.staff_settings.workers;
    let Some(draft) = state.draft.as_ref() else {
        return container(centered_state("选择一个 Worker，或新建注册配置。"))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(settings_panel_style)
            .into();
    };

    let save = action_button(
        if state.saving { "保存中" } else { "保存 Worker" },
        has_project && !state.loading && !state.saving,
        StaffResourceMessage::WorkerSaveRequested,
        true,
    );
    let fields = column![
        row![
            column![
                text(if state.selected_worker_id.is_some() {
                    "编辑 Worker"
                } else {
                    "新增 Worker"
                })
                .size(15),
                text("仅注册配置可编辑；生命周期、心跳和网关时间字段只读。")
                    .size(11)
                    .style(settings_muted_text_style),
            ]
            .spacing(4)
            .width(Length::Fill),
            save,
        ]
        .spacing(12)
        .align_y(Alignment::Center),
        settings_section_card("注册配置", "保存后通过项目 Agent Scheduler 网关 upsert。"),
        worker_text_field(
            "Worker ID",
            "项目内稳定标识。",
            &draft.worker_id,
            StaffWorkerDraftField::WorkerId
        ),
        worker_text_field(
            "Runtime",
            "运行时名称，可留空。",
            &draft.runtime,
            StaffWorkerDraftField::Runtime
        ),
        worker_text_field(
            "Deployment",
            "部署标识，可留空。",
            &draft.deployment,
            StaffWorkerDraftField::Deployment
        ),
        worker_text_field(
            "Model",
            "Worker 使用的模型。",
            &draft.model,
            StaffWorkerDraftField::Model
        ),
        worker_text_field(
            "Model Provider",
            "模型提供方，可留空。",
            &draft.model_provider,
            StaffWorkerDraftField::ModelProvider,
        ),
        worker_text_field(
            "Team ID",
            "所属 Team，可留空。",
            &draft.team_id,
            StaffWorkerDraftField::TeamId
        ),
        worker_text_field(
            "Worker Role",
            "Team 内角色。",
            &draft.worker_role,
            StaffWorkerDraftField::WorkerRole,
        ),
        worker_text_field(
            "Matrix User ID",
            "远程 Worker 的 Matrix 用户标识，可留空。",
            &draft.matrix_user_id,
            StaffWorkerDraftField::MatrixUserId,
        ),
        worker_text_field(
            "Room ID",
            "远程 Worker 房间，可留空。",
            &draft.room_id,
            StaffWorkerDraftField::RoomId
        ),
        capabilities_editor(draft),
        lifecycle_snapshot(draft),
    ]
    .spacing(12);

    container(
        scrollable(container(fields).padding(iced::Padding::default().right(12.0)))
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
            .height(Length::Fill),
    )
    .padding([16, 18])
    .width(Length::Fill)
    .height(Length::Fill)
    .style(settings_panel_style)
    .into()
}

fn worker_text_field<'a>(
    label: &'static str,
    hint: &'static str,
    value: &'a str,
    field: StaffWorkerDraftField,
) -> Element<'a, Message> {
    row![
        field_label(label, hint),
        text_input("", value)
            .on_input(move |value| {
                resource_message(StaffResourceMessage::WorkerFieldChanged(field, value))
            })
            .padding([9, 11])
            .size(13)
            .style(settings_text_input_style)
            .width(Length::Fill),
    ]
    .spacing(16)
    .align_y(Alignment::Center)
    .into()
}

fn capabilities_editor(draft: &StaffWorkerDraft) -> Element<'_, Message> {
    let mut rows = column![settings_section_card(
        "Capabilities",
        "按类型与名称维护结构化能力；既有 metadata 在编辑时保留。",
    )]
    .spacing(10);

    if draft.capabilities.is_empty() {
        rows = rows.push(text("暂无能力声明。").size(12).style(settings_muted_text_style));
    }

    for (index, capability) in draft.capabilities.iter().enumerate() {
        let kind_values: Vec<String> =
            CAPABILITY_KINDS.iter().map(|(_, key)| (*key).to_string()).collect();
        let selected_kind = capability_kind_key(&capability.kind).to_string();
        let kind_source = draft.capabilities.clone();
        let name_source = draft.capabilities.clone();
        let remove_source = draft.capabilities.clone();
        rows = rows.push(
            row![
                pick_list(kind_values, Some(selected_kind), move |value| {
                    let mut capabilities = kind_source.clone();
                    capabilities[index].kind = capability_kind_from_key(&value);
                    resource_message(StaffResourceMessage::WorkerCapabilitiesChanged(capabilities))
                })
                .width(Length::Fixed(150.0)),
                text_input("能力名称", &capability.name)
                    .on_input(move |value| {
                        let mut capabilities = name_source.clone();
                        capabilities[index].name = value;
                        resource_message(StaffResourceMessage::WorkerCapabilitiesChanged(
                            capabilities,
                        ))
                    })
                    .padding([9, 11])
                    .size(13)
                    .style(settings_text_input_style)
                    .width(Length::Fill),
                button(text("移除").size(12))
                    .padding([8, 12])
                    .on_press({
                        let mut capabilities = remove_source;
                        capabilities.remove(index);
                        resource_message(StaffResourceMessage::WorkerCapabilitiesChanged(
                            capabilities,
                        ))
                    })
                    .style(rounded_action_btn_style),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        );
    }

    let mut capabilities = draft.capabilities.clone();
    capabilities.push(AgentCapabilityDto {
        kind: AgentCapabilityKindDto::Tool,
        name: String::new(),
        metadata: Default::default(),
    });
    rows.push(
        button(text("添加 Capability").size(12))
            .padding([8, 14])
            .on_press(resource_message(StaffResourceMessage::WorkerCapabilitiesChanged(
                capabilities,
            )))
            .style(rounded_action_btn_style),
    )
    .into()
}

fn lifecycle_snapshot(draft: &StaffWorkerDraft) -> Element<'_, Message> {
    column![
        settings_section_card("运行快照（只读）", "这些字段由网关生命周期与心跳更新。"),
        readonly_row("Status", draft.status.as_ref().map(|value| format!("{value:?}"))),
        readonly_row("Desired State", draft.desired_state.clone()),
        readonly_row("Container State", draft.container_state.clone()),
        readonly_row("Idle Since", draft.idle_since_ms.map(|value| value.0.to_string())),
        readonly_row("Last Seen", draft.last_seen_at_ms.map(|value| value.0.to_string())),
        readonly_row("Last Heartbeat", draft.last_heartbeat_at.clone()),
        readonly_row("Updated At", draft.updated_at_ms.map(|value| value.0.to_string())),
    ]
    .spacing(8)
    .into()
}

fn readonly_row(label: &'static str, value: Option<String>) -> Element<'static, Message> {
    row![
        text(label).size(12).width(Length::Fixed(150.0)),
        text(value.filter(|value| !value.trim().is_empty()).unwrap_or_else(|| "—".to_string()))
            .size(12)
            .style(settings_muted_text_style),
    ]
    .spacing(14)
    .align_y(Alignment::Center)
    .into()
}

fn field_label(label: &'static str, hint: &'static str) -> Element<'static, Message> {
    column![text(label).size(12), text(hint).size(11).style(settings_muted_text_style)]
        .spacing(3)
        .width(Length::Fixed(190.0))
        .into()
}

fn centered_state(message: &'static str) -> Element<'static, Message> {
    container(text(message).size(12).style(settings_muted_text_style))
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .into()
}

fn action_button(
    label: &'static str,
    enabled: bool,
    message: StaffResourceMessage,
    primary: bool,
) -> Element<'static, Message> {
    let button = button(text(label).size(12)).padding([8, 14]).style(if primary {
        primary_action_btn_style
    } else {
        rounded_action_btn_style
    });
    if enabled { button.on_press(resource_message(message)).into() } else { button.into() }
}

fn resource_message(message: StaffResourceMessage) -> Message {
    Message::Settings(SettingsMessage::Staff(StaffMessage::Resources(message)))
}

fn capability_kind_key(kind: &AgentCapabilityKindDto) -> &'static str {
    match kind {
        AgentCapabilityKindDto::Skill => "skill",
        AgentCapabilityKindDto::RemoteSkill => "remote_skill",
        AgentCapabilityKindDto::McpServer => "mcp_server",
        AgentCapabilityKindDto::Model => "model",
        AgentCapabilityKindDto::Runtime => "runtime",
        AgentCapabilityKindDto::Tool => "tool",
    }
}

fn capability_kind_from_key(value: &str) -> AgentCapabilityKindDto {
    match value {
        "skill" => AgentCapabilityKindDto::Skill,
        "remote_skill" => AgentCapabilityKindDto::RemoteSkill,
        "mcp_server" => AgentCapabilityKindDto::McpServer,
        "model" => AgentCapabilityKindDto::Model,
        "runtime" => AgentCapabilityKindDto::Runtime,
        _ => AgentCapabilityKindDto::Tool,
    }
}

#[cfg(test)]
#[path = "workers_tests.rs"]
mod workers_tests;
