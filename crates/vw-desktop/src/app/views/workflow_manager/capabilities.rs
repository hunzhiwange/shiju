//! 工作流管理中的阶段能力目录面板。

use iced::widget::{Space, button, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Element, Length};

use crate::app::components::system_settings_common::{
    danger_action_btn_style, primary_action_btn_style, rounded_action_btn_style,
    settings_error_banner, settings_muted_text_style, settings_panel, settings_section_card,
    settings_text_input_style, settings_value_badge,
};
use crate::app::message::TaskBoardMessage;
use crate::app::task::staff::stage_capabilities::{
    ADVANCE_STATUS_CAPABILITY_ID, CODE_REVIEW_CAPABILITY_ID, EXECUTE_TASK_CAPABILITY_ID,
    GIT_MERGE_CAPABILITY_ID, MANUAL_CAPABILITY_ID, SAVED_WORKFLOW_CAPABILITY_ID, StageCapability,
    stage_capability_registry,
};
use crate::app::task::{
    TaskBoardStaffCapability, TaskBoardStaffCapabilityDraftField, TaskBoardStaffCapabilityKind,
    TaskBoardStaffCapabilitySummary, TaskBoardStaffCapabilitySummaryKind,
};
use crate::app::{App, Message};

const CAPABILITY_SCROLL_RIGHT_GUTTER: f32 = 12.0;

#[derive(Clone, Copy)]
struct BuiltinCapabilityPanelItem {
    id: &'static str,
    label: &'static str,
    input_contract: &'static str,
    result_semantics: &'static str,
    permission_hint: &'static str,
    requires_workflow_uuid: bool,
}

fn stage_capability_by_stable_id(id: &str) -> Option<&'static StageCapability> {
    stage_capability_registry().all().iter().find(|capability| capability.stable_id == id)
}

fn builtin_stage_capability_item(
    capability_id: &'static str,
    label: &'static str,
) -> Option<BuiltinCapabilityPanelItem> {
    let capability = stage_capability_by_stable_id(capability_id)?;
    Some(BuiltinCapabilityPanelItem {
        id: capability.stable_id,
        label,
        input_contract: capability.input_contract,
        result_semantics: capability.result_semantics,
        permission_hint: capability.permission_hint,
        requires_workflow_uuid: capability.requires_workflow_uuid,
    })
}

fn builtin_capability_panel_items() -> Vec<BuiltinCapabilityPanelItem> {
    let mut items = Vec::new();
    if let Some(item) = builtin_stage_capability_item(EXECUTE_TASK_CAPABILITY_ID, "默认 Agent 执行")
    {
        items.push(item);
    }
    items.push(BuiltinCapabilityPanelItem {
        id: "task_split",
        label: "任务拆分",
        input_contract: "任务提示词、拆分策略、目标粒度",
        result_semantics: "生成子任务草案，并保留原任务上下文",
        permission_hint: "只写入任务拆分结果，不扩大执行或文件系统权限。",
        requires_workflow_uuid: false,
    });
    for (capability_id, label) in [
        (CODE_REVIEW_CAPABILITY_ID, "代码审核"),
        (GIT_MERGE_CAPABILITY_ID, "Git 合并"),
        (ADVANCE_STATUS_CAPABILITY_ID, "推进状态"),
        (MANUAL_CAPABILITY_ID, "人工处理"),
        (SAVED_WORKFLOW_CAPABILITY_ID, "已保存 Workflow"),
    ] {
        if let Some(item) = builtin_stage_capability_item(capability_id, label) {
            items.push(item);
        }
    }
    items
}

fn capability_kind_label(kind: TaskBoardStaffCapabilityKind) -> &'static str {
    match kind {
        TaskBoardStaffCapabilityKind::BuiltIn => "内置能力",
        TaskBoardStaffCapabilityKind::SavedWorkflow => "已保存 Workflow",
    }
}

fn fields_from_summary_input(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|field| !field.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn capability_binding_summary() -> &'static str {
    "执行结果按任务流自动推进：成功进入当前工作流成功后续；失败进入失败分支；阻塞保留阶段并提示人工处理。"
}

fn build_builtin_capability_card(item: BuiltinCapabilityPanelItem) -> Element<'static, Message> {
    let workflow_badge = if item.requires_workflow_uuid {
        settings_value_badge("需要 Workflow UUID")
    } else {
        settings_value_badge("无需 Workflow UUID")
    };
    settings_panel(
        column![
            row![
                text(item.label)
                    .size(13)
                    .font(iced::Font { weight: iced::font::Weight::Bold, ..Default::default() })
                    .width(Length::Fill),
                settings_value_badge("内置只读"),
                settings_value_badge(item.id),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
            row![settings_value_badge("类型：内置"), workflow_badge]
                .spacing(6)
                .align_y(Alignment::Center),
            text(format!("输入契约：{}", item.input_contract))
                .size(11)
                .style(settings_muted_text_style),
            text(format!("结果语义：{}", item.result_semantics))
                .size(11)
                .style(settings_muted_text_style),
            text(format!("权限提示：{}", item.permission_hint))
                .size(11)
                .style(settings_muted_text_style),
            text(capability_binding_summary()).size(11).style(settings_muted_text_style),
        ]
        .spacing(8)
        .width(Length::Fill),
    )
    .into()
}

fn build_capability_summary_editor<'a>(
    capability_id: &'a str,
    title: &'static str,
    summary_kind: TaskBoardStaffCapabilitySummaryKind,
    summary: &'a TaskBoardStaffCapabilitySummary,
) -> Element<'a, Message> {
    let fields_text = summary.fields.join(", ");
    let description_for_fields = summary.description.clone();
    let fields_for_description = summary.fields.clone();
    let capability_id_for_description = capability_id.to_string();
    let capability_id_for_fields = capability_id.to_string();

    column![
        text(title)
            .size(12)
            .font(iced::Font { weight: iced::font::Weight::Bold, ..Default::default() }),
        text_input("摘要描述", &summary.description)
            .on_input(move |description| {
                Message::TaskBoard(TaskBoardMessage::UpdateStaffCapabilitySummaryDraft {
                    capability_id: capability_id_for_description.clone(),
                    summary: summary_kind,
                    description,
                    fields: fields_for_description.clone(),
                })
            })
            .padding([6, 10])
            .size(12)
            .style(settings_text_input_style),
        text_input("字段，使用英文逗号分隔", &fields_text)
            .on_input(move |value| {
                Message::TaskBoard(TaskBoardMessage::UpdateStaffCapabilitySummaryDraft {
                    capability_id: capability_id_for_fields.clone(),
                    summary: summary_kind,
                    description: description_for_fields.clone(),
                    fields: fields_from_summary_input(&value),
                })
            })
            .padding([6, 10])
            .size(12)
            .style(settings_text_input_style),
    ]
    .spacing(6)
    .width(Length::Fill)
    .into()
}

fn build_custom_capability_card<'a>(
    capability: &'a TaskBoardStaffCapability,
) -> Element<'a, Message> {
    let id_for_id_field = capability.id.clone();
    let id_for_label_field = capability.id.clone();
    let id_for_delete = capability.id.clone();
    let type_label = capability_kind_label(capability.capability_type);
    let workflow_label = if capability.requires_workflow_uuid {
        "需要 Workflow UUID"
    } else {
        "无需 Workflow UUID"
    };

    settings_panel(
        column![
            row![
                text(&capability.label)
                    .size(13)
                    .font(iced::Font { weight: iced::font::Weight::Bold, ..Default::default() })
                    .width(Length::Fill),
                settings_value_badge("自定义可编辑"),
                settings_value_badge(type_label),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
            row![
                text_input("能力 id", &capability.id)
                    .on_input(move |value| {
                        Message::TaskBoard(TaskBoardMessage::UpdateStaffCapabilityDraft {
                            capability_id: id_for_id_field.clone(),
                            field: TaskBoardStaffCapabilityDraftField::Id,
                            value,
                        })
                    })
                    .padding([6, 10])
                    .size(12)
                    .style(settings_text_input_style)
                    .width(Length::FillPortion(1)),
                text_input("能力名称", &capability.label)
                    .on_input(move |value| {
                        Message::TaskBoard(TaskBoardMessage::UpdateStaffCapabilityDraft {
                            capability_id: id_for_label_field.clone(),
                            field: TaskBoardStaffCapabilityDraftField::Label,
                            value,
                        })
                    })
                    .padding([6, 10])
                    .size(12)
                    .style(settings_text_input_style)
                    .width(Length::FillPortion(1)),
            ]
            .spacing(8),
            row![
                settings_value_badge(format!("类型：{type_label}")),
                settings_value_badge(workflow_label)
            ]
            .spacing(6)
            .align_y(Alignment::Center),
            build_capability_summary_editor(
                &capability.id,
                "输入摘要",
                TaskBoardStaffCapabilitySummaryKind::Input,
                &capability.input_summary,
            ),
            build_capability_summary_editor(
                &capability.id,
                "结果摘要",
                TaskBoardStaffCapabilitySummaryKind::Result,
                &capability.result_summary,
            ),
            text(capability_binding_summary()).size(11).style(settings_muted_text_style),
            row![
                Space::new().width(Length::Fill),
                button(text("删除自定义能力").size(12))
                    .on_press(Message::TaskBoard(TaskBoardMessage::DeleteStaffCapabilityDraft {
                        capability_id: id_for_delete,
                    }))
                    .padding([5, 10])
                    .style(danger_action_btn_style),
            ]
            .align_y(Alignment::Center),
        ]
        .spacing(9)
        .width(Length::Fill),
    )
    .into()
}

pub(super) fn build_capability_management_view(app: &App) -> Element<'_, Message> {
    let workflow_id = super::workflow_manager_stage_action_workflow_id(app);
    let catalog_ready = app.task_board_staff_capability_catalog_workflow_id.as_deref()
        == Some(workflow_id.as_str())
        && !app.task_board_staff_capability_catalog.is_empty();
    let reload_workflow_id = workflow_id.clone();
    let save_workflow_id = workflow_id;
    let custom_capabilities = if catalog_ready {
        app.task_board_staff_capability_draft
            .iter()
            .filter(|capability| capability.editable())
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let loading_label = if app.task_board_staff_capability_catalog_loading || !catalog_ready {
        "加载中"
    } else {
        "目录就绪"
    };
    let saving_label =
        if app.task_board_staff_capability_saving { "保存中" } else { "可保存" };

    let mut save_button = button(
        text(if app.task_board_staff_capability_saving {
            "保存中"
        } else {
            "保存能力目录"
        })
        .size(12),
    )
    .padding([6, 12])
    .style(primary_action_btn_style);
    if catalog_ready && !app.task_board_staff_capability_saving {
        save_button = save_button.on_press(Message::TaskBoard(
            TaskBoardMessage::SaveStaffCapabilityCatalogForWorkflow {
                workflow_id: save_workflow_id,
            },
        ));
    }

    let mut add_button =
        button(text("新增自定义能力").size(12)).padding([6, 12]).style(rounded_action_btn_style);
    if catalog_ready {
        add_button =
            add_button.on_press(Message::TaskBoard(TaskBoardMessage::AddStaffCapabilityDraft));
    }

    let actions = row![
        settings_value_badge(format!("内置 {}", builtin_capability_panel_items().len())),
        settings_value_badge(format!("自定义 {}", custom_capabilities.len())),
        settings_value_badge(loading_label),
        settings_value_badge(saving_label),
        Space::new().width(Length::Fill),
        button(text("重新加载").size(12))
            .on_press(Message::TaskBoard(TaskBoardMessage::LoadStaffCapabilityCatalogForWorkflow {
                workflow_id: reload_workflow_id,
            },))
            .padding([6, 12])
            .style(rounded_action_btn_style),
        add_button,
        save_button,
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let mut content = column![
        settings_section_card("能力管理", "统一查看内置能力，并维护项目级自定义能力目录。",),
        actions,
    ]
    .spacing(12)
    .width(Length::Fill);

    if let Some(error) = app.task_board_staff_capability_catalog_error.as_deref() {
        content = content.push(settings_error_banner(error));
    }

    content = content.push(
        text("内置能力")
            .size(13)
            .font(iced::Font { weight: iced::font::Weight::Bold, ..Default::default() }),
    );
    for item in builtin_capability_panel_items() {
        content = content.push(build_builtin_capability_card(item));
    }

    content = content.push(
        text("自定义能力")
            .size(13)
            .font(iced::Font { weight: iced::font::Weight::Bold, ..Default::default() }),
    );
    if !catalog_ready {
        content = content.push(settings_panel(
            text("正在加载当前工作流能力目录").size(12).style(settings_muted_text_style),
        ));
    } else if custom_capabilities.is_empty() {
        content = content.push(settings_panel(
            text("暂无自定义能力；点击新增后会创建一个已保存 Workflow 类型的能力草稿。")
                .size(12)
                .style(settings_muted_text_style),
        ));
    } else {
        for capability in custom_capabilities {
            content = content.push(build_custom_capability_card(capability));
        }
    }

    settings_panel(
        scrollable(
            container(content)
                .width(Length::Fill)
                .padding(iced::Padding::default().right(CAPABILITY_SCROLL_RIGHT_GUTTER)),
        )
        .direction(iced::widget::scrollable::Direction::Vertical(
            iced::widget::scrollable::Scrollbar::new().width(4).scroller_width(4),
        ))
        .height(Length::Fill),
    )
    .height(Length::Fill)
    .into()
}

#[cfg(test)]
#[path = "capabilities_tests.rs"]
mod capabilities_tests;
