//! 系统设置员工配置页。

mod teams;
mod workers;

use crate::app::components::system_settings_common::{
    danger_action_btn_style, primary_action_btn_style, rounded_action_btn_style,
    settings_checkbox_style, settings_error_banner, settings_muted_text_style, settings_page_intro,
    settings_panel_style, settings_pick_list_menu_style, settings_pick_list_style,
    settings_section_card, settings_segment_button_style, settings_success_banner,
    settings_text_input_style, settings_value_badge,
};
use crate::app::components::system_settings_search::{
    SettingsTabDescriptor, StaffSettingsSearchTab,
};
use crate::app::message::settings::{SettingsMessage, StaffMessage};
use crate::app::state::{
    SkillsCatalogItem, StaffSettingsCapabilitiesDraft, StaffSettingsChannelBindingDraft,
    StaffSettingsMemberDraft, StaffSettingsTab, is_builtin_staff_member_id,
    staff_member_needs_agent_binding,
};
use crate::app::{App, Message};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{
    button, checkbox, column, container, pick_list, row, scrollable, text, text_input,
};
use iced::{Alignment, Background, Border, Color, Element, Length, Shadow, Theme, Vector};
use vw_config_types::agent::is_retired_builtin_agent_key;

const UNBOUND_AGENT_LABEL: &str = "不绑定";
const STAFF_TAB_ROSTER: &str = "名册";
const STAFF_TAB_CAPABILITIES: &str = "能力";
const STAFF_TAB_TEAMS: &str = "Teams";
const STAFF_TAB_WORKERS: &str = "Workers";

const STAFF_SEARCH_TABS: &[SettingsTabDescriptor<StaffSettingsSearchTab>] = &[
    SettingsTabDescriptor::new(
        StaffSettingsSearchTab::Roster,
        STAFF_TAB_ROSTER,
        &["roster", "员工", "名册", "agent binding", "绑定"],
    ),
    SettingsTabDescriptor::new(
        StaffSettingsSearchTab::Capabilities,
        STAFF_TAB_CAPABILITIES,
        &["capability", "能力", "工具", "技能"],
    ),
    SettingsTabDescriptor::new(
        StaffSettingsSearchTab::Teams,
        STAFF_TAB_TEAMS,
        &["team", "团队", "编排"],
    ),
    SettingsTabDescriptor::new(
        StaffSettingsSearchTab::Workers,
        STAFF_TAB_WORKERS,
        &["worker", "执行器", "任务"],
    ),
];

pub(super) fn search_tabs() -> &'static [SettingsTabDescriptor<StaffSettingsSearchTab>] {
    STAFF_SEARCH_TABS
}
const CUSTOM_OPTION_PREFIX: &str = "自定义：";
const DEFAULT_CHANNEL_INSTANCE_ID: &str = "default";
const STAFF_CHANNEL_TYPE_OPTIONS: &[(&str, &str)] =
    &[("Matrix", "matrix"), ("Feishu", "feishu"), ("DingTalk", "dingtalk")];
const STAFF_ROLE_OPTIONS: &[(&str, &str)] = &[
    ("编排", "orchestrator"),
    ("规划", "planner"),
    ("执行", "executor"),
    ("评审", "reviewer"),
    ("合并", "merger"),
    ("观察", "observer"),
    ("成员", "member"),
];
const STAFF_LEVEL_OPTIONS: &[(&str, &str)] =
    &[("负责人", "principal"), ("主管", "manager"), ("执行者", "executor"), ("成员", "member")];
const STAFF_STATUS_OPTIONS: &[(&str, &str)] =
    &[("在线", "online"), ("工作中", "working"), ("阻塞", "blocked"), ("离线", "offline")];

#[derive(Debug, Clone, PartialEq, Eq)]
struct RosterMemberSummary {
    member_id: String,
    display_name: String,
    role_label: String,
    status_label: String,
    capacity_label: String,
    binding_label: String,
    shows_unbound_warning: bool,
    enabled: bool,
    is_builtin: bool,
}

fn roster_member_summaries(app: &App) -> Vec<RosterMemberSummary> {
    app.staff_settings
        .member_drafts
        .iter()
        .map(|draft| {
            let display_name =
                if draft.name.trim().is_empty() { draft.id.clone() } else { draft.name.clone() };

            RosterMemberSummary {
                member_id: draft.id.clone(),
                display_name,
                role_label: staff_option_label(STAFF_ROLE_OPTIONS, &draft.role, "未设置角色"),
                status_label: staff_option_label(STAFF_STATUS_OPTIONS, &draft.status, "未设置状态"),
                capacity_label: format!("容量 {}", trim_or_fallback(&draft.capacity_input, "0")),
                binding_label: binding_label(draft),
                shows_unbound_warning: selected_member_save_blocked(draft),
                enabled: draft.enabled,
                is_builtin: is_builtin_staff_member_id(&draft.id),
            }
        })
        .collect()
}

fn selected_member_save_blocked(draft: &StaffSettingsMemberDraft) -> bool {
    staff_member_needs_agent_binding(draft)
}

pub fn view(app: &App) -> Element<'_, Message> {
    let rows = roster_member_summaries(app);
    let row_count = rows.len();
    let selected_id = selected_member_id(app);
    let selected = selected_member_draft(app);
    let active_tab = app.staff_settings.active_tab;
    let save_blocked = app.staff_settings.member_drafts.iter().any(selected_member_save_blocked);
    let has_unsaved_changes = app.staff_settings.has_unsaved_changes();

    let intro = settings_page_intro(
        "员工配置",
        "管理任务看板员工名册、执行 agent 绑定和后续能力配置入口。",
    );

    let inner_tabs = row![
        staff_tab_button(StaffSettingsTab::Roster, active_tab),
        staff_tab_button(StaffSettingsTab::Capabilities, active_tab),
        staff_tab_button(StaffSettingsTab::Teams, active_tab),
        staff_tab_button(StaffSettingsTab::Workers, active_tab),
    ]
    .spacing(10)
    .wrap();

    let status_row = row![
        settings_value_badge(format!("{} 名员工", row_count)),
        settings_value_badge(if app.staff_settings.loading { "同步中" } else { "已就绪" }),
        settings_value_badge(if save_blocked { "存在未绑定" } else { "绑定完整" }),
    ]
    .spacing(10)
    .wrap();

    let mut content = column![intro, inner_tabs].spacing(14).height(Length::Fill);

    let main_panel = match active_tab {
        StaffSettingsTab::Teams => teams::view(app),
        StaffSettingsTab::Workers => workers::view(app),
        StaffSettingsTab::Roster | StaffSettingsTab::Capabilities => {
            content = content.push(status_row);
            if app.staff_settings.save_success {
                content = content.push(settings_success_banner("员工配置已保存。"));
            }
            if save_blocked {
                content = content.push(settings_error_banner(
                    "启用的员工必须绑定委托代理或 ACP agent 后才能保存。",
                ));
            }
            row![
                roster_panel(rows, selected_id),
                detail_panel(app, selected, active_tab, save_blocked, has_unsaved_changes)
            ]
            .spacing(20)
            .height(Length::Fill)
            .into()
        }
    };

    content = content.push(main_panel);

    content.width(Length::Fill).height(Length::Fill).into()
}

fn selected_member_id(app: &App) -> Option<String> {
    app.staff_settings
        .selected_member_id
        .as_ref()
        .filter(|member_id| {
            app.staff_settings.member_drafts.iter().any(|draft| &draft.id == *member_id)
        })
        .cloned()
        .or_else(|| app.staff_settings.member_drafts.first().map(|draft| draft.id.clone()))
}

fn selected_member_draft(app: &App) -> Option<&StaffSettingsMemberDraft> {
    let selected_id = selected_member_id(app);
    selected_id
        .as_deref()
        .and_then(|member_id| {
            app.staff_settings.member_drafts.iter().find(|draft| draft.id == member_id)
        })
        .or_else(|| app.staff_settings.member_drafts.first())
}

fn roster_panel(
    rows: Vec<RosterMemberSummary>,
    selected_id: Option<String>,
) -> Element<'static, Message> {
    let add_button = button(text("新增员工").size(12))
        .padding([8, 14])
        .on_press(Message::Settings(SettingsMessage::Staff(StaffMessage::AddMemberRequested)))
        .style(rounded_action_btn_style);

    let roster_items: Element<'static, Message> = if rows.is_empty() {
        column![text("暂无员工").size(12).style(settings_muted_text_style)].spacing(8).into()
    } else {
        column(
            rows.into_iter()
                .map(|member| {
                    let selected = selected_id.as_deref() == Some(member.member_id.as_str());
                    roster_row(member, selected)
                })
                .collect::<Vec<_>>(),
        )
        .spacing(8)
        .into()
    };

    container(
        column![
            row![
                column![
                    text("名册").size(14),
                    text("选择员工后编辑基础字段与 agent 绑定。")
                        .size(12)
                        .style(settings_muted_text_style),
                ]
                .spacing(4),
                container(add_button).align_x(iced::alignment::Horizontal::Right),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
            scrollable(container(roster_items).padding(iced::Padding::default().right(10.0)))
                .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
                .height(Length::Fill),
        ]
        .spacing(12)
        .height(Length::Fill),
    )
    .padding([18, 18])
    .width(Length::Fixed(284.0))
    .height(Length::Fill)
    .style(settings_panel_style)
    .into()
}

fn roster_row(member: RosterMemberSummary, selected: bool) -> Element<'static, Message> {
    let warning = member.shows_unbound_warning;
    let member_id = member.member_id.clone();
    let mut meta =
        format!("{} | {} | {}", member.role_label, member.status_label, member.capacity_label);
    if !member.enabled {
        meta.push_str(" | 停用");
    }

    let content = column![
        row![
            text(member.display_name).size(14),
            container(text(if member.is_builtin { "内置" } else { "自定义" }).size(11))
                .padding([3, 7])
                .style(move |theme: &Theme| badge_style(theme, selected)),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .wrap(),
        text(meta).size(12).style(settings_muted_text_style),
        text(member.binding_label).size(12).style(move |theme: &Theme| {
            iced::widget::text::Style {
                color: Some(if warning {
                    Color::from_rgb8(255, 191, 105)
                } else {
                    theme.palette().text.scale_alpha(0.70)
                }),
            }
        }),
    ]
    .spacing(5)
    .width(Length::Fill);

    button(container(content).padding([10, 12]).width(Length::Fill))
        .width(Length::Fill)
        .on_press(Message::Settings(SettingsMessage::Staff(StaffMessage::SelectMember(member_id))))
        .style(move |theme: &Theme, status| roster_button_style(theme, status, selected, warning))
        .into()
}

fn detail_panel<'a>(
    app: &'a App,
    selected: Option<&'a StaffSettingsMemberDraft>,
    active_tab: StaffSettingsTab,
    save_blocked: bool,
    has_unsaved_changes: bool,
) -> Element<'a, Message> {
    let Some(draft) = selected else {
        return container(text("暂无可编辑员工").size(13).style(settings_muted_text_style))
            .padding([18, 20])
            .width(Length::Fill)
            .height(Length::Fill)
            .style(settings_panel_style)
            .into();
    };

    let member_id = draft.id.clone();
    let member_title = trim_or_fallback(&draft.name, &draft.id);
    let delete_button = delete_member_button(&draft);
    let reset_button = button(text("重置").size(12))
        .padding([8, 14])
        .on_press(Message::Settings(SettingsMessage::Staff(StaffMessage::ResetDraftRequested)))
        .style(rounded_action_btn_style);
    let save_button =
        save_member_button(save_blocked || app.staff_settings.loading || !has_unsaved_changes);

    let delegate_options = delegate_agent_options(app, &draft.delegate_agent);
    let acp_options = acp_agent_options(app, &draft.acp_agent);
    let selected_delegate = selected_agent_label(&draft.delegate_agent);
    let selected_acp = selected_agent_label(&draft.acp_agent);
    let selected_warning = selected_member_save_blocked(&draft);

    let mut fields = match active_tab {
        StaffSettingsTab::Roster => roster_detail_fields(
            app,
            &member_id,
            draft,
            delegate_options,
            selected_delegate,
            acp_options,
            selected_acp,
        ),
        StaffSettingsTab::Capabilities => capability_detail_fields(app, draft),
        StaffSettingsTab::Teams | StaffSettingsTab::Workers => {
            unreachable!("resource tabs use panels")
        }
    };

    if selected_warning {
        fields = fields.push(settings_error_banner(
            "该员工已启用但未绑定 agent；保存前请选择委托代理或 ACP agent。",
        ));
    }

    container(
        column![
            row![
                column![
                    text(member_title).size(16),
                    text(format!("成员 ID: {}", draft.id))
                        .size(12)
                        .style(settings_muted_text_style),
                ]
                .spacing(4),
                container(row![delete_button, reset_button, save_button].spacing(10).wrap())
                    .align_x(iced::alignment::Horizontal::Right),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
            scrollable(container(fields).padding(iced::Padding::default().right(12.0)))
                .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
                .height(Length::Fill),
        ]
        .spacing(14)
        .height(Length::Fill),
    )
    .padding([18, 20])
    .width(Length::Fill)
    .height(Length::Fill)
    .style(settings_panel_style)
    .into()
}

fn roster_detail_fields<'a>(
    app: &'a App,
    member_id: &str,
    draft: &'a StaffSettingsMemberDraft,
    delegate_options: Vec<String>,
    selected_delegate: String,
    acp_options: Vec<String>,
    selected_acp: String,
) -> iced::widget::Column<'a, Message> {
    column![
        settings_section_card(
            "基础信息",
            "这些字段写入 gateway staff 配置，用于任务看板员工名册展示。",
        ),
        text_field_row("名称", "员工显示名称。", &draft.name, {
            let member_id = member_id.to_string();
            move |value| StaffMessage::MemberNameChanged(member_id.clone(), value)
        }),
        pick_field_row(
            "角色",
            "任务看板中匹配阶段所有者的角色 key。",
            staff_option_labels(STAFF_ROLE_OPTIONS, &draft.role, "未设置角色"),
            staff_option_label(STAFF_ROLE_OPTIONS, &draft.role, "未设置角色"),
            {
                let member_id = member_id.to_string();
                move |value| {
                    StaffMessage::MemberRoleChanged(
                        member_id.clone(),
                        staff_option_value_from_label(STAFF_ROLE_OPTIONS, value, "未设置角色"),
                    )
                }
            },
        ),
        pick_field_row(
            "层级",
            "用于展示和后续调度排序的层级 key。",
            staff_option_labels(STAFF_LEVEL_OPTIONS, &draft.level, "未设置层级"),
            staff_option_label(STAFF_LEVEL_OPTIONS, &draft.level, "未设置层级"),
            {
                let member_id = member_id.to_string();
                move |value| {
                    StaffMessage::MemberLevelChanged(
                        member_id.clone(),
                        staff_option_value_from_label(STAFF_LEVEL_OPTIONS, value, "未设置层级"),
                    )
                }
            },
        ),
        pick_field_row(
            "状态",
            "稳定状态 key，任务看板会映射到本地展示。",
            staff_option_labels(STAFF_STATUS_OPTIONS, &draft.status, "未设置状态"),
            staff_option_label(STAFF_STATUS_OPTIONS, &draft.status, "未设置状态"),
            {
                let member_id = member_id.to_string();
                move |value| {
                    StaffMessage::MemberStatusChanged(
                        member_id.clone(),
                        staff_option_value_from_label(STAFF_STATUS_OPTIONS, value, "未设置状态"),
                    )
                }
            },
        ),
        text_field_row(
            "容量",
            "该员工可同时承接的任务数。",
            &draft.capacity_input,
            {
                let member_id = member_id.to_string();
                move |value| StaffMessage::MemberCapacityChanged(member_id.clone(), value)
            }
        ),
        row![
            label_col("启用", "关闭后保留员工但不参与接单。"),
            checkbox(draft.enabled)
                .label("允许参与员工模式")
                .on_toggle({
                    let member_id = member_id.to_string();
                    move |value| {
                        Message::Settings(SettingsMessage::Staff(
                            StaffMessage::MemberEnabledToggled(member_id.clone(), value),
                        ))
                    }
                })
                .style(settings_checkbox_style),
        ]
        .spacing(16)
        .align_y(Alignment::Center),
        settings_section_card("Agent 绑定", "每名启用员工至少绑定一个可执行 agent。",),
        pick_field_row(
            "委托代理",
            "绑定 Delegate Agent key。",
            delegate_options,
            selected_delegate,
            {
                let member_id = member_id.to_string();
                move |value| {
                    StaffMessage::MemberDelegateAgentChanged(
                        member_id.clone(),
                        agent_value_from_label(value),
                    )
                }
            },
        ),
        pick_field_row("ACP agent", "绑定已启用的 ACP agent。", acp_options, selected_acp, {
            let member_id = member_id.to_string();
            move |value| {
                StaffMessage::MemberAcpAgentChanged(
                    member_id.clone(),
                    agent_value_from_label(value),
                )
            }
        },),
        channel_bindings_section(app, member_id, draft),
    ]
    .spacing(14)
}

fn channel_bindings_section<'a>(
    app: &'a App,
    member_id: &str,
    draft: &'a StaffSettingsMemberDraft,
) -> Element<'a, Message> {
    let profile_options = channel_binding_profile_options(app, draft);
    let profile_refs: Element<'a, Message> = if profile_options.is_empty() {
        text("暂无已配置 profile，可手动填写实例 ID。")
            .size(12)
            .style(settings_muted_text_style)
            .into()
    } else {
        row(profile_options.into_iter().map(settings_value_badge).collect::<Vec<_>>())
            .spacing(8)
            .wrap()
            .into()
    };

    let binding_rows: Element<'a, Message> = if draft.channel_bindings.is_empty() {
        text("未绑定外部通道实例。").size(12).style(settings_muted_text_style).into()
    } else {
        column(
            draft
                .channel_bindings
                .iter()
                .enumerate()
                .map(|(index, binding)| channel_binding_row(member_id, index, binding))
                .collect::<Vec<_>>(),
        )
        .spacing(10)
        .into()
    };

    let add_button = button(text("新增通道绑定").size(12))
        .padding([8, 14])
        .on_press(Message::Settings(SettingsMessage::Staff(StaffMessage::ChannelBindingAdded(
            member_id.to_string(),
        ))))
        .style(rounded_action_btn_style);

    column![
        settings_section_card(
            "通道绑定",
            "为员工按通道类型绑定稳定实例 ID；可用 profile ref 来自通道设置。",
        ),
        settings_value_badge(channel_binding_summary_label(draft)),
        profile_refs,
        binding_rows,
        add_button,
    ]
    .spacing(10)
    .into()
}

fn channel_binding_row<'a>(
    member_id: &str,
    index: usize,
    binding: &'a StaffSettingsChannelBindingDraft,
) -> Element<'a, Message> {
    let member_id_for_type = member_id.to_string();
    let member_id_for_instance = member_id.to_string();
    let member_id_for_remove = member_id.to_string();
    let type_options =
        staff_option_labels(STAFF_CHANNEL_TYPE_OPTIONS, &binding.channel_type, "未设置通道");
    let selected_type =
        staff_option_label(STAFF_CHANNEL_TYPE_OPTIONS, &binding.channel_type, "未设置通道");

    container(
        row![
            pick_list(type_options, Some(selected_type), move |value| {
                Message::Settings(SettingsMessage::Staff(
                    StaffMessage::ChannelBindingChannelTypeChanged(
                        member_id_for_type.clone(),
                        index,
                        staff_option_value_from_label(
                            STAFF_CHANNEL_TYPE_OPTIONS,
                            value,
                            "未设置通道",
                        ),
                    ),
                ))
            })
            .style(settings_pick_list_style)
            .menu_style(settings_pick_list_menu_style)
            .width(Length::Fixed(150.0)),
            text_input("", &binding.instance_id)
                .on_input(move |value| {
                    Message::Settings(SettingsMessage::Staff(
                        StaffMessage::ChannelBindingInstanceIdChanged(
                            member_id_for_instance.clone(),
                            index,
                            value,
                        ),
                    ))
                })
                .padding([10, 12])
                .size(13)
                .style(settings_text_input_style)
                .width(Length::Fill),
            button(text("删除").size(12))
                .padding([8, 12])
                .on_press(Message::Settings(SettingsMessage::Staff(
                    StaffMessage::ChannelBindingRemoved(member_id_for_remove, index),
                )))
                .style(danger_action_btn_style),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    )
    .padding([10, 12])
    .width(Length::Fill)
    .style(|theme: &Theme| capability_choice_style(theme, false))
    .into()
}

fn capability_detail_fields<'a>(
    app: &'a App,
    draft: &'a StaffSettingsMemberDraft,
) -> iced::widget::Column<'a, Message> {
    let Some(capabilities) = app
        .staff_settings
        .capability_drafts
        .iter()
        .find(|candidate| candidate.member_id == draft.id)
    else {
        return column![settings_error_banner("能力草稿缺失，请刷新员工配置后重试。")].spacing(14);
    };

    let member_id = capabilities.member_id.clone();
    let available_tools = normalized_ids(&app.agents_settings.available_tools);
    let available_skill_ids =
        available_skill_items(app).into_iter().map(|skill| skill.id.clone()).collect::<Vec<_>>();
    let effective_tool_count =
        effective_selected_count(&capabilities.allowed_tools, &available_tools);
    let effective_skill_count =
        effective_selected_count(&capabilities.allowed_skills, &available_skill_ids);
    let limits_label = format!(
        "迭代 {} | 并发 {} | 时长 {}",
        trim_or_fallback(&capabilities.max_iterations_input, "未设置"),
        optional_limit_label(&capabilities.max_parallel_tasks_input),
        optional_limit_label(&capabilities.max_active_minutes_input),
    );

    column![
        settings_section_card("能力摘要", "这些工具、技能和运行约束只写入 gateway staff 配置。",),
        row![
            settings_value_badge(format!("有效工具 {effective_tool_count}")),
            settings_value_badge(format!("有效技能 {effective_skill_count}")),
            settings_value_badge(limits_label),
        ]
        .spacing(10)
        .wrap(),
        row![
            label_col("Agentic", "允许该员工使用自己的工具循环。"),
            checkbox(capabilities.agentic)
                .label("启用员工级 agentic 能力")
                .on_toggle({
                    let member_id = member_id.clone();
                    move |enabled| {
                        Message::Settings(SettingsMessage::Staff(
                            StaffMessage::CapabilityAgenticToggled(member_id.clone(), enabled),
                        ))
                    }
                })
                .style(settings_checkbox_style),
        ]
        .spacing(16)
        .align_y(Alignment::Center),
        text_field_row(
            "最大迭代",
            "员工 agentic 工具循环的最大迭代次数。",
            &capabilities.max_iterations_input,
            {
                let member_id = member_id.clone();
                move |value| StaffMessage::CapabilityMaxIterationsChanged(member_id.clone(), value)
            },
        ),
        text_field_row(
            "最大并发",
            "可留空；限制该员工同时活跃任务数。",
            &capabilities.max_parallel_tasks_input,
            {
                let member_id = member_id.clone();
                move |value| {
                    StaffMessage::CapabilityMaxParallelTasksChanged(member_id.clone(), value)
                }
            },
        ),
        text_field_row(
            "最大时长",
            "可留空；限制该员工单次活跃分钟数。",
            &capabilities.max_active_minutes_input,
            {
                let member_id = member_id.clone();
                move |value| {
                    StaffMessage::CapabilityMaxActiveMinutesChanged(member_id.clone(), value)
                }
            },
        ),
        capability_tools_section(app, capabilities),
        capability_skills_section(app, capabilities),
    ]
    .spacing(14)
}

fn capability_tools_section<'a>(
    app: &'a App,
    capabilities: &'a StaffSettingsCapabilitiesDraft,
) -> Element<'a, Message> {
    let tools = normalized_ids(&app.agents_settings.available_tools);
    let selected = normalized_ids(&capabilities.allowed_tools);
    let selected_count =
        selected.iter().filter(|tool| tools.iter().any(|available| available == *tool)).count();

    let list: Element<'a, Message> = if tools.is_empty() {
        text("暂无可用工具列表").size(12).style(settings_muted_text_style).into()
    } else {
        row(tools
            .iter()
            .map(|tool| {
                let checked = selected.iter().any(|selected| selected == tool);
                let member_id = capabilities.member_id.clone();
                let tool_id = tool.clone();
                capability_choice(
                    tool.clone(),
                    "工具 allowlist".to_string(),
                    checked,
                    move |enabled| {
                        StaffMessage::CapabilityAllowedToolToggled(
                            member_id.clone(),
                            tool_id.clone(),
                            enabled,
                        )
                    },
                )
            })
            .collect::<Vec<_>>())
        .spacing(10)
        .wrap()
        .into()
    };

    column![
        settings_section_card("允许工具", "从 gateway 提供的工具清单中选择该员工可用项。",),
        row![
            settings_value_badge(format!("已选 {selected_count}/{}", tools.len())),
            settings_value_badge(format!("草稿 {}", selected.len())),
        ]
        .spacing(10)
        .wrap(),
        list,
    ]
    .spacing(10)
    .into()
}

fn capability_skills_section<'a>(
    app: &'a App,
    capabilities: &'a StaffSettingsCapabilitiesDraft,
) -> Element<'a, Message> {
    let skills = available_skill_items(app);
    let available_skill_ids = skills.iter().map(|skill| skill.id.clone()).collect::<Vec<_>>();
    let selected = normalized_ids(&capabilities.allowed_skills);
    let selected_count = selected
        .iter()
        .filter(|skill| available_skill_ids.iter().any(|available| available == *skill))
        .count();

    let list: Element<'a, Message> = if skills.is_empty() {
        column![
            text(if app.skills_settings.loading {
                "正在刷新技能目录 ..."
            } else {
                "当前暂无已启用技能"
            })
            .size(12)
            .style(settings_muted_text_style),
            button(text("刷新技能").size(12))
                .padding([8, 14])
                .on_press(Message::Settings(SettingsMessage::SkillsRefresh))
                .style(rounded_action_btn_style),
        ]
        .spacing(10)
        .into()
    } else {
        row(skills
            .into_iter()
            .map(|skill| {
                let checked = selected.iter().any(|selected| selected == &skill.id);
                let member_id = capabilities.member_id.clone();
                let skill_id = skill.id.clone();
                capability_choice(skill.title.clone(), skill.id.clone(), checked, move |enabled| {
                    StaffMessage::CapabilityAllowedSkillToggled(
                        member_id.clone(),
                        skill_id.clone(),
                        enabled,
                    )
                })
            })
            .collect::<Vec<_>>())
        .spacing(10)
        .wrap()
        .into()
    };

    column![
        settings_section_card("允许技能", "从已安装且启用的技能 catalog 中选择该员工可用项。",),
        row![
            settings_value_badge(format!("已选 {selected_count}/{}", available_skill_ids.len())),
            settings_value_badge(format!("草稿 {}", selected.len())),
        ]
        .spacing(10)
        .wrap(),
        list,
    ]
    .spacing(10)
    .into()
}

fn capability_choice<'a>(
    label: String,
    detail: String,
    checked: bool,
    message: impl Fn(bool) -> StaffMessage + 'a,
) -> Element<'a, Message> {
    container(
        column![
            checkbox(checked)
                .label(label)
                .on_toggle(move |enabled| {
                    Message::Settings(SettingsMessage::Staff(message(enabled)))
                })
                .style(settings_checkbox_style),
            text(detail).size(11).style(settings_muted_text_style),
        ]
        .spacing(6)
        .width(Length::Fill),
    )
    .padding([10, 12])
    .width(Length::Fixed(220.0))
    .style(move |theme: &Theme| capability_choice_style(theme, checked))
    .into()
}

fn available_skill_items(app: &App) -> Vec<&SkillsCatalogItem> {
    let mut skills = app
        .skills_settings
        .catalog
        .iter()
        .filter(|skill| skill.installed && skill.enabled)
        .collect::<Vec<_>>();
    skills.sort_by(|left, right| left.id.cmp(&right.id));
    skills
}

fn normalized_ids(values: &[String]) -> Vec<String> {
    let mut values = values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values
}

fn effective_selected_count(selected: &[String], available: &[String]) -> usize {
    normalized_ids(selected)
        .iter()
        .filter(|selected| available.iter().any(|available| available == *selected))
        .count()
}

fn optional_limit_label(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() { "不限".to_string() } else { value.to_string() }
}

fn staff_tab_button<'a>(
    tab: StaffSettingsTab,
    active_tab: StaffSettingsTab,
) -> Element<'a, Message> {
    let active = tab == active_tab;
    let label = staff_tab_label(tab);
    button(text(label).size(13))
        .padding([8, 16])
        .on_press(Message::Settings(SettingsMessage::Staff(StaffMessage::TabChanged(
            tab.key().to_string(),
        ))))
        .style(move |theme: &Theme, status| settings_segment_button_style(theme, status, active))
        .into()
}

fn staff_tab_label(tab: StaffSettingsTab) -> &'static str {
    match tab {
        StaffSettingsTab::Roster => STAFF_TAB_ROSTER,
        StaffSettingsTab::Capabilities => STAFF_TAB_CAPABILITIES,
        StaffSettingsTab::Teams => STAFF_TAB_TEAMS,
        StaffSettingsTab::Workers => STAFF_TAB_WORKERS,
    }
}

fn text_field_row<'a>(
    label: &'a str,
    hint: &'a str,
    value: &'a str,
    message: impl Fn(String) -> StaffMessage + 'a,
) -> Element<'a, Message> {
    row![
        label_col(label, hint),
        text_input("", value)
            .on_input(move |value| Message::Settings(SettingsMessage::Staff(message(value))))
            .padding([10, 12])
            .size(13)
            .style(settings_text_input_style)
            .width(Length::Fill),
    ]
    .spacing(16)
    .align_y(Alignment::Center)
    .into()
}

fn pick_field_row<'a>(
    label: &'a str,
    hint: &'a str,
    options: Vec<String>,
    selected: String,
    message: impl Fn(String) -> StaffMessage + 'a,
) -> Element<'a, Message> {
    row![
        label_col(label, hint),
        pick_list(options, Some(selected), move |value| {
            Message::Settings(SettingsMessage::Staff(message(value)))
        })
        .style(settings_pick_list_style)
        .menu_style(settings_pick_list_menu_style)
        .width(Length::Fill),
    ]
    .spacing(16)
    .align_y(Alignment::Center)
    .into()
}

fn label_col<'a>(title: &'a str, hint: &'a str) -> iced::widget::Column<'a, Message> {
    column![text(title).size(13), text(hint).size(12).style(settings_muted_text_style)]
        .spacing(4)
        .width(Length::Fixed(180.0))
}

fn delete_member_button<'a>(draft: &StaffSettingsMemberDraft) -> Element<'a, Message> {
    if is_builtin_staff_member_id(&draft.id) {
        return button(text("内置").size(12))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .into();
    }

    button(text("删除").size(12))
        .padding([8, 14])
        .on_press(Message::Settings(SettingsMessage::Staff(StaffMessage::DeleteMemberRequested(
            draft.id.clone(),
        ))))
        .style(danger_action_btn_style)
        .into()
}

fn save_member_button<'a>(disabled: bool) -> Element<'a, Message> {
    let button = button(text("保存").size(12)).padding([8, 16]).style(save_action_btn_style);
    if disabled {
        button.into()
    } else {
        button
            .on_press(Message::Settings(SettingsMessage::Staff(StaffMessage::SaveRequested)))
            .into()
    }
}

fn save_action_btn_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    if matches!(status, iced::widget::button::Status::Disabled) {
        let palette = theme.extended_palette();
        return iced::widget::button::Style {
            background: Some(Background::Color(palette.background.strong.color.scale_alpha(0.34))),
            text_color: palette.background.strong.text.scale_alpha(0.48),
            border: Border {
                radius: 8.0.into(),
                width: 1.0,
                color: palette.background.strong.color.scale_alpha(0.24),
            },
            ..Default::default()
        };
    }

    primary_action_btn_style(theme, status)
}

fn delegate_agent_options(app: &App, selected: &Option<String>) -> Vec<String> {
    let mut options = Vec::with_capacity(app.agents_settings.entries.len() + 2);
    options.push(UNBOUND_AGENT_LABEL.to_string());
    options.extend(
        app.agents_settings
            .entries
            .iter()
            .filter(|entry| !is_retired_builtin_agent_key(&entry.key))
            .map(|entry| entry.key.clone()),
    );
    if selected.as_deref().is_none_or(|key| !is_retired_builtin_agent_key(key)) {
        push_selected_agent(&mut options, selected);
    }
    options.sort();
    options.dedup();
    keep_unbound_first(options)
}

fn acp_agent_options(app: &App, selected: &Option<String>) -> Vec<String> {
    let mut options = Vec::with_capacity(app.acp_agents.len() + 2);
    options.push(UNBOUND_AGENT_LABEL.to_string());
    options.extend(app.acp_agents.iter().cloned());
    push_selected_agent(&mut options, selected);
    options.sort();
    options.dedup();
    keep_unbound_first(options)
}

fn push_selected_agent(options: &mut Vec<String>, selected: &Option<String>) {
    if let Some(value) = selected.as_deref().map(str::trim).filter(|value| !value.is_empty()) {
        options.push(value.to_string());
    }
}

fn channel_binding_profile_options(app: &App, draft: &StaffSettingsMemberDraft) -> Vec<String> {
    let mut options = Vec::new();
    if app.channels_settings.matrix.is_some() {
        push_channel_profile_option(&mut options, "matrix", DEFAULT_CHANNEL_INSTANCE_ID);
    }
    for instance in &app.channels_settings.matrix_instances {
        push_channel_profile_option(&mut options, "matrix", &instance.id);
    }
    if app.channels_settings.feishu.is_some() {
        push_channel_profile_option(&mut options, "feishu", DEFAULT_CHANNEL_INSTANCE_ID);
    }
    for instance in &app.channels_settings.feishu_instances {
        push_channel_profile_option(&mut options, "feishu", &instance.id);
    }
    if app.channels_settings.dingtalk.is_some() {
        push_channel_profile_option(&mut options, "dingtalk", DEFAULT_CHANNEL_INSTANCE_ID);
    }
    for instance in &app.channels_settings.dingtalk_instances {
        push_channel_profile_option(&mut options, "dingtalk", &instance.id);
    }
    for binding in &draft.channel_bindings {
        push_channel_profile_option(&mut options, &binding.channel_type, &binding.instance_id);
    }
    options.sort();
    options.dedup();
    options
}

fn push_channel_profile_option(options: &mut Vec<String>, channel_type: &str, instance_id: &str) {
    let channel_type = channel_type.trim();
    let instance_id = instance_id.trim();
    if channel_type.is_empty() || instance_id.is_empty() {
        return;
    }
    options.push(format!("{channel_type}:{instance_id}"));
}

fn channel_binding_summary_label(draft: &StaffSettingsMemberDraft) -> String {
    let labels = draft
        .channel_bindings
        .iter()
        .map(|binding| {
            format!(
                "{} {}",
                channel_type_display_label(&binding.channel_type),
                trim_or_fallback(&binding.instance_id, DEFAULT_CHANNEL_INSTANCE_ID),
            )
        })
        .collect::<Vec<_>>();
    if labels.is_empty() { "未绑定外部通道".to_string() } else { labels.join(", ") }
}

fn channel_type_display_label(channel_type: &str) -> String {
    let channel_type = channel_type.trim();
    STAFF_CHANNEL_TYPE_OPTIONS
        .iter()
        .find(|(_, value)| *value == channel_type)
        .map(|(label, _)| (*label).to_string())
        .unwrap_or_else(|| format!("{CUSTOM_OPTION_PREFIX}{channel_type}"))
}

fn keep_unbound_first(mut options: Vec<String>) -> Vec<String> {
    options.retain(|option| option != UNBOUND_AGENT_LABEL);
    let mut ordered = vec![UNBOUND_AGENT_LABEL.to_string()];
    ordered.extend(options);
    ordered
}

fn staff_option_labels(
    options: &[(&str, &str)],
    selected_value: &str,
    empty_label: &str,
) -> Vec<String> {
    let mut labels = options.iter().map(|(label, _)| (*label).to_string()).collect::<Vec<_>>();
    let selected = staff_option_label(options, selected_value, empty_label);
    if !labels.iter().any(|label| label == &selected) {
        labels.push(selected);
    }
    labels
}

fn staff_option_label(options: &[(&str, &str)], value: &str, empty_label: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        return empty_label.to_string();
    }

    options
        .iter()
        .find(|(_, option_value)| *option_value == value)
        .map(|(label, _)| (*label).to_string())
        .unwrap_or_else(|| format!("{CUSTOM_OPTION_PREFIX}{value}"))
}

fn staff_option_value_from_label(
    options: &[(&str, &str)],
    label: String,
    empty_label: &str,
) -> String {
    let label = label.trim();
    if label == empty_label {
        return String::new();
    }

    options
        .iter()
        .find(|(option_label, _)| *option_label == label)
        .map(|(_, value)| (*value).to_string())
        .unwrap_or_else(|| {
            label.strip_prefix(CUSTOM_OPTION_PREFIX).unwrap_or(label).trim().to_string()
        })
}

fn selected_agent_label(value: &Option<String>) -> String {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| UNBOUND_AGENT_LABEL.to_string())
}

fn agent_value_from_label(value: String) -> Option<String> {
    let value = value.trim();
    if value.is_empty() || value == UNBOUND_AGENT_LABEL { None } else { Some(value.to_string()) }
}

fn binding_label(draft: &StaffSettingsMemberDraft) -> String {
    if let Some(agent) =
        draft.delegate_agent.as_deref().map(str::trim).filter(|value| !value.is_empty())
    {
        format!("委托代理 {agent}")
    } else if let Some(agent) =
        draft.acp_agent.as_deref().map(str::trim).filter(|value| !value.is_empty())
    {
        format!("ACP agent {agent}")
    } else {
        "未绑定 agent".to_string()
    }
}

fn trim_or_fallback(value: &str, fallback: &str) -> String {
    let value = value.trim();
    if value.is_empty() { fallback.to_string() } else { value.to_string() }
}

fn capability_choice_style(theme: &Theme, checked: bool) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);
    iced::widget::container::Style {
        text_color: Some(theme.palette().text),
        background: Some(Background::Color(if checked {
            theme.palette().primary.scale_alpha(if is_dark { 0.14 } else { 0.08 })
        } else if is_dark {
            palette.background.base.color.scale_alpha(0.30)
        } else {
            Color::WHITE.scale_alpha(0.58)
        })),
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: if checked {
                theme.palette().primary.scale_alpha(if is_dark { 0.34 } else { 0.24 })
            } else if is_dark {
                palette.background.strong.color.scale_alpha(0.58)
            } else {
                Color::from_rgba8(15, 23, 42, 0.06)
            },
        },
        ..Default::default()
    }
}

fn roster_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
    selected: bool,
    warning: bool,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);
    let background = if selected {
        Some(Background::Color(theme.palette().primary.scale_alpha(if is_dark {
            0.16
        } else {
            0.08
        })))
    } else {
        match status {
            iced::widget::button::Status::Hovered => Some(Background::Color(if is_dark {
                palette.background.weak.color.scale_alpha(0.62)
            } else {
                Color::WHITE.scale_alpha(0.76)
            })),
            iced::widget::button::Status::Pressed => Some(Background::Color(if is_dark {
                palette.background.strong.color.scale_alpha(0.84)
            } else {
                palette.background.weak.color.scale_alpha(0.92)
            })),
            _ => Some(Background::Color(if is_dark {
                palette.background.base.color.scale_alpha(0.30)
            } else {
                Color::WHITE.scale_alpha(0.52)
            })),
        }
    };

    let border_color = if warning {
        Color::from_rgb8(255, 191, 105).scale_alpha(if selected { 0.75 } else { 0.48 })
    } else if selected {
        theme.palette().primary.scale_alpha(if is_dark { 0.34 } else { 0.22 })
    } else if is_dark {
        palette.background.strong.color.scale_alpha(0.58)
    } else {
        Color::from_rgba8(15, 23, 42, 0.06)
    };

    iced::widget::button::Style {
        background,
        text_color: theme.palette().text,
        border: Border { radius: 12.0.into(), width: 1.0, color: border_color },
        shadow: if selected {
            Shadow {
                color: Color::BLACK.scale_alpha(if is_dark { 0.16 } else { 0.06 }),
                offset: Vector::new(0.0, 4.0),
                blur_radius: 10.0,
            }
        } else {
            Shadow::default()
        },
        ..Default::default()
    }
}

fn badge_style(theme: &Theme, selected: bool) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);
    iced::widget::container::Style {
        text_color: Some(if selected {
            theme.palette().primary
        } else {
            theme.palette().text.scale_alpha(0.72)
        }),
        background: Some(Background::Color(if selected {
            theme.palette().primary.scale_alpha(if is_dark { 0.16 } else { 0.08 })
        } else if is_dark {
            palette.background.weak.color.scale_alpha(0.34)
        } else {
            Color::from_rgba8(248, 250, 252, 0.90)
        })),
        border: Border {
            radius: 999.0.into(),
            width: 1.0,
            color: if selected {
                theme.palette().primary.scale_alpha(0.24)
            } else if is_dark {
                palette.background.strong.color.scale_alpha(0.62)
            } else {
                Color::from_rgba8(15, 23, 42, 0.06)
            },
        },
        ..Default::default()
    }
}

fn is_dark_theme(theme: &Theme) -> bool {
    let background = theme.palette().background;
    background.r + background.g + background.b < 1.5
}

#[cfg(test)]
mod tests;
