//! 员工设置中的独立 Team 网关资源管理面板。

use crate::app::components::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style, settings_checkbox_style,
    settings_error_banner, settings_muted_text_style, settings_panel_style,
    settings_pick_list_menu_style, settings_pick_list_style, settings_section_card,
    settings_segment_button_style, settings_success_banner, settings_text_input_style,
    settings_value_badge,
};
use crate::app::message::settings::{
    SettingsMessage, StaffMessage, StaffResourceMessage, StaffTeamDraftField,
    StaffTeamMemberDraftField,
};
use crate::app::{App, Message};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{
    button, checkbox, column, container, pick_list, row, scrollable, text, text_input,
};
use iced::{Alignment, Element, Length, Theme};
use vw_gateway_client::vw_api_types::agent_scheduler::AgentTeamStatusDto;

const TEAM_STATUSES: &[(&str, &str)] =
    &[("待就绪", "pending"), ("活跃", "active"), ("降级", "degraded"), ("失败", "failed")];

pub(super) fn view(app: &App) -> Element<'_, Message> {
    let state = &app.staff_settings.teams;
    let has_project = app.project_path.as_deref().is_some_and(|path| !path.trim().is_empty());

    let refresh = action_button(
        if state.loading { "刷新中" } else { "刷新" },
        !state.loading && has_project,
        StaffResourceMessage::Refresh,
        true,
    );
    let add = action_button(
        "新增 Team",
        !state.loading && !state.saving && has_project,
        StaffResourceMessage::TeamNewRequested,
        false,
    );

    let mut panel = column![
        row![
            column![
                text("Teams").size(16),
                text("经 GatewayClient 管理项目 Team 快照与结构化成员。")
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
            settings_value_badge(format!("{} Teams", state.teams.len())),
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

    panel = panel
        .push(row![team_list(app), team_detail(app, has_project)].spacing(20).height(Length::Fill));

    container(panel.height(Length::Fill))
        .padding([18, 20])
        .width(Length::Fill)
        .height(Length::Fill)
        .style(settings_panel_style)
        .into()
}

fn team_list(app: &App) -> Element<'_, Message> {
    let state = &app.staff_settings.teams;
    let rows: Element<'_, Message> = if state.loading && state.teams.is_empty() {
        centered_state("正在加载 Team…")
    } else if state.teams.is_empty() {
        centered_state("暂无 Team；可新建项目级团队。")
    } else {
        column(
            state
                .teams
                .iter()
                .map(|team| {
                    let selected = state.selected_team_id.as_deref() == Some(team.id.as_str());
                    let team_id = team.id.clone();
                    button(
                        column![
                            text(team.name.clone()).size(13),
                            text(format!(
                                "{} · {:?} · {}/{} 就绪",
                                team.id, team.status, team.ready_workers, team.total_workers
                            ))
                            .size(11)
                            .style(settings_muted_text_style),
                        ]
                        .spacing(4)
                        .width(Length::Fill),
                    )
                    .padding([10, 12])
                    .width(Length::Fill)
                    .on_press(resource_message(StaffResourceMessage::TeamSelected(team_id)))
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
            text("Team 列表").size(14),
            scrollable(container(rows).padding(iced::Padding::default().right(10.0)))
                .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
                .height(Length::Fill),
        ]
        .spacing(12)
        .height(Length::Fill),
    )
    .padding([16, 16])
    .width(Length::Fixed(300.0))
    .height(Length::Fill)
    .style(settings_panel_style)
    .into()
}

fn team_detail(app: &App, has_project: bool) -> Element<'_, Message> {
    let state = &app.staff_settings.teams;
    let Some(draft) = state.draft.as_ref() else {
        return container(centered_state("选择一个 Team，或新建团队。"))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(settings_panel_style)
            .into();
    };

    let save = action_button(
        if state.saving { "保存中" } else { "保存 Team" },
        has_project && !state.loading && !state.saving,
        StaffResourceMessage::TeamSaveRequested,
        true,
    );
    let status_options: Vec<String> =
        TEAM_STATUSES.iter().map(|(label, _)| (*label).to_string()).collect();
    let selected_status = team_status_label(&draft.status).to_string();
    let fields = column![
        row![
            column![
                text(if state.selected_team_id.is_some() { "编辑 Team" } else { "新增 Team" })
                    .size(15),
                text("成员、Leader 与就绪状态会共同生成一致的 Team 计数。")
                    .size(11)
                    .style(settings_muted_text_style),
            ]
            .spacing(4)
            .width(Length::Fill),
            save,
        ]
        .spacing(12)
        .align_y(Alignment::Center),
        settings_section_card("Team 配置", "保存后通过项目 Agent Scheduler 网关 upsert。"),
        team_text_field("Team ID", "项目内稳定标识。", &draft.team_id, StaffTeamDraftField::TeamId),
        team_text_field("名称", "团队显示名称。", &draft.name, StaffTeamDraftField::Name),
        row![
            field_label("状态", "Team 当前管理状态。"),
            pick_list(status_options, Some(selected_status), |value| {
                resource_message(StaffResourceMessage::TeamStatusChanged(team_status_from_label(
                    &value,
                )))
            })
            .style(settings_pick_list_style)
            .menu_style(settings_pick_list_menu_style)
            .width(Length::Fill),
        ]
        .spacing(16)
        .align_y(Alignment::Center),
        team_text_field(
            "Leader",
            "必须引用下方成员 Worker ID。",
            &draft.leader_id,
            StaffTeamDraftField::LeaderId
        ),
        team_text_field(
            "Team Room",
            "团队共享房间，可留空。",
            &draft.team_room_id,
            StaffTeamDraftField::TeamRoomId,
        ),
        team_text_field(
            "Leader DM Room",
            "Leader 私聊房间，可留空。",
            &draft.leader_dm_room_id,
            StaffTeamDraftField::LeaderDmRoomId,
        ),
        team_members_editor(app),
        row![
            settings_value_badge(format!("{} 成员", draft.members.len())),
            settings_value_badge(format!(
                "{} 已就绪",
                draft.members.iter().filter(|member| member.ready).count()
            )),
            settings_value_badge(
                draft
                    .updated_at_ms
                    .map(|value| format!("更新于 {}", value.0))
                    .unwrap_or_else(|| "尚未保存".to_string())
            ),
        ]
        .spacing(10)
        .wrap(),
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

fn team_members_editor(app: &App) -> Element<'_, Message> {
    let draft = app.staff_settings.teams.draft.as_ref().expect("team draft exists");
    let loaded_worker_ids = app
        .staff_settings
        .workers
        .workers
        .iter()
        .map(|worker| worker.id.clone())
        .collect::<Vec<_>>();
    let mut rows = column![settings_section_card(
        "结构化成员",
        "成员必须引用已加载 Worker；就绪计数由这些开关计算。",
    )]
    .spacing(10);

    if draft.members.is_empty() {
        rows = rows.push(
            text("暂无成员。请先刷新 Worker，再添加成员。")
                .size(12)
                .style(settings_muted_text_style),
        );
    }

    for (index, member) in draft.members.iter().enumerate() {
        let mut worker_options = loaded_worker_ids.clone();
        if !worker_options.iter().any(|candidate| candidate == &member.worker_id) {
            worker_options.push(member.worker_id.clone());
        }
        worker_options.sort();
        worker_options.dedup();
        let selected_worker = member.worker_id.clone();
        rows = rows.push(
            container(
                column![
                    row![
                        text(format!("成员 {}", index + 1)).size(13).width(Length::Fill),
                        button(text("移除").size(12))
                            .padding([7, 12])
                            .on_press(resource_message(StaffResourceMessage::TeamMemberRemoved(
                                index,
                            )))
                            .style(rounded_action_btn_style),
                    ]
                    .align_y(Alignment::Center),
                    member_pick_field(index, worker_options, selected_worker),
                    member_text_field(
                        index,
                        "Runtime Name",
                        &member.runtime_name,
                        StaffTeamMemberDraftField::RuntimeName,
                    ),
                    member_text_field(index, "Role", &member.role, StaffTeamMemberDraftField::Role),
                    member_text_field(
                        index,
                        "Room ID",
                        &member.room_id,
                        StaffTeamMemberDraftField::RoomId,
                    ),
                    member_text_field(
                        index,
                        "Matrix User ID",
                        &member.matrix_user_id,
                        StaffTeamMemberDraftField::MatrixUserId,
                    ),
                    checkbox(member.ready)
                        .label("Worker 已就绪")
                        .on_toggle(move |ready| {
                            resource_message(StaffResourceMessage::TeamMemberReadyToggled(
                                index, ready,
                            ))
                        })
                        .style(settings_checkbox_style),
                ]
                .spacing(9),
            )
            .padding([12, 14])
            .width(Length::Fill)
            .style(settings_panel_style),
        );
    }

    let existing = draft.members.iter().map(|member| member.worker_id.as_str()).collect::<Vec<_>>();
    let available = loaded_worker_ids
        .into_iter()
        .filter(|worker_id| !existing.iter().any(|existing| *existing == worker_id))
        .collect::<Vec<_>>();
    if available.is_empty() {
        rows = rows
            .push(text("没有可添加的已加载 Worker。").size(11).style(settings_muted_text_style));
    } else {
        rows = rows.push(
            row![
                text("添加成员").size(12),
                pick_list(available, None::<String>, |worker_id| {
                    resource_message(StaffResourceMessage::TeamMemberAdded(worker_id))
                })
                .placeholder("选择 Worker")
                .style(settings_pick_list_style)
                .menu_style(settings_pick_list_menu_style)
                .width(Length::Fixed(240.0)),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        );
    }

    rows.into()
}

fn team_text_field<'a>(
    label: &'static str,
    hint: &'static str,
    value: &'a str,
    field: StaffTeamDraftField,
) -> Element<'a, Message> {
    row![
        field_label(label, hint),
        text_input("", value)
            .on_input(move |value| {
                resource_message(StaffResourceMessage::TeamFieldChanged(field, value))
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

fn member_pick_field(
    index: usize,
    options: Vec<String>,
    selected: String,
) -> Element<'static, Message> {
    row![
        text("Worker ID").size(12).width(Length::Fixed(150.0)),
        pick_list(options, Some(selected), move |value| {
            resource_message(StaffResourceMessage::TeamMemberFieldChanged(
                index,
                StaffTeamMemberDraftField::WorkerId,
                value,
            ))
        })
        .style(settings_pick_list_style)
        .menu_style(settings_pick_list_menu_style)
        .width(Length::Fill),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .into()
}

fn member_text_field<'a>(
    index: usize,
    label: &'static str,
    value: &'a str,
    field: StaffTeamMemberDraftField,
) -> Element<'a, Message> {
    row![
        text(label).size(12).width(Length::Fixed(150.0)),
        text_input("", value)
            .on_input(move |value| {
                resource_message(StaffResourceMessage::TeamMemberFieldChanged(index, field, value))
            })
            .padding([8, 10])
            .size(12)
            .style(settings_text_input_style)
            .width(Length::Fill),
    ]
    .spacing(12)
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

fn team_status_label(status: &AgentTeamStatusDto) -> &'static str {
    match status {
        AgentTeamStatusDto::Pending => "待就绪",
        AgentTeamStatusDto::Active => "活跃",
        AgentTeamStatusDto::Degraded => "降级",
        AgentTeamStatusDto::Failed => "失败",
    }
}

fn team_status_from_label(value: &str) -> AgentTeamStatusDto {
    match value {
        "活跃" => AgentTeamStatusDto::Active,
        "降级" => AgentTeamStatusDto::Degraded,
        "失败" => AgentTeamStatusDto::Failed,
        _ => AgentTeamStatusDto::Pending,
    }
}

#[cfg(test)]
#[path = "teams_tests.rs"]
mod teams_tests;
