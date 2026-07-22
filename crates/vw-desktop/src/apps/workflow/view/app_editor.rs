//! 工作流应用编辑器视图，负责应用名称、图标、描述和请求限制的表单渲染。

use super::*;
use iced::widget::{column, row};

/// 构建或更新 build app editor modal 相关行为。
///
/// 参数由调用方提供，返回值直接交给上层视图或状态处理；本函数不执行额外错误恢复。
pub(super) fn build_app_editor_modal(state: &WorkflowState) -> Element<'_, Message> {
    let Some(editor) = state.app_editor.as_ref() else {
        return container(Space::new().width(1).height(1)).into();
    };

    const APP_EDITOR_MODAL_WIDTH: f32 = 680.0;
    const APP_EDITOR_MODAL_HEIGHT: f32 = 620.0;
    const APP_EDITOR_SCROLLBAR_WIDTH: f32 = 4.0;
    const APP_EDITOR_SCROLL_GUTTER: f32 = 12.0;

    let title = match &editor.mode {
        super::state::WorkflowAppEditorMode::Create => "新增应用",
        super::state::WorkflowAppEditorMode::Edit(_) => "编辑应用信息",
    };

    let subtitle = match &editor.mode {
        super::state::WorkflowAppEditorMode::Create => {
            "创建一个新的 workflow 应用，后续可以单独保存成 yml 文件。"
        }
        super::state::WorkflowAppEditorMode::Edit(_) => {
            "修改应用元数据，不会丢失原始 Dify yml 里的节点业务字段。"
        }
    };

    let header = row![
        row![
            column![
                text(title).size(24),
                text(subtitle).size(12).style(settings_muted_text_style),
            ]
            .spacing(4),
            Space::new().width(Length::Fill),
            settings_close_button(Message::WorkflowTool(WorkflowMessage::CloseAppEditor)),
        ]
        .align_y(Alignment::Center),
    ];

    let mut content = column![section_card(
        "应用基础信息",
        "表单结构参考 Dify 应用设置页，先整理名称、图标和描述。",
    )]
    .spacing(16);

    if let super::state::WorkflowAppEditorMode::Edit(target_id) = &editor.mode {
        content = content.push(build_app_uuid_field(state, target_id));
        if state.active_app_id.as_deref() == Some(target_id.as_str()) {
            content = content.push(build_app_organize_field());
        }
    }

    let content = content
        .push(build_editor_field(
            "应用名称",
            workflow_text_input("例如：客服分流工作流", &editor.name, |value| {
                Message::WorkflowTool(WorkflowMessage::AppEditorNameChanged(value))
            }),
        ))
        .push(build_editor_field(
            "应用图标",
            workflow_text_input("例如：🤖", &editor.icon, |value| {
                Message::WorkflowTool(WorkflowMessage::AppEditorIconChanged(value))
            }),
        ))
        .push(build_editor_field(
            "应用描述",
            workflow_text_input(
                "描述这个 workflow 用来做什么",
                &editor.description,
                |value| Message::WorkflowTool(WorkflowMessage::AppEditorDescriptionChanged(value)),
            ),
        ))
        .push(build_editor_field(
            "使用 web app 图标替换 🤖",
            row![
                toggler(editor.use_icon_as_answer_icon).on_toggle(|value| {
                    Message::WorkflowTool(WorkflowMessage::AppEditorUseIconAsAnswerIconChanged(
                        value,
                    ))
                }),
                text("在分享和 Explore 场景里用应用图标替换默认机器人图标")
                    .size(12)
                    .style(settings_muted_text_style),
            ]
            .spacing(10)
            .align_y(Alignment::Center)
            .into(),
        ))
        .push(build_editor_field(
            "最大活跃请求数",
            workflow_text_input("0 表示不限制", &editor.max_active_requests_input, |value| {
                Message::WorkflowTool(WorkflowMessage::AppEditorMaxActiveRequestsChanged(value))
            }),
        ));

    let footer = row![
        Space::new().width(Length::Fill),
        button(text("取消"))
            .style(rounded_action_btn_style)
            .padding([9, 14])
            .on_press(Message::WorkflowTool(WorkflowMessage::CloseAppEditor)),
        button(text("保存应用信息"))
            .style(primary_action_btn_style)
            .padding([9, 14])
            .on_press(Message::WorkflowTool(WorkflowMessage::SubmitAppEditor)),
    ]
    .spacing(10);

    let scroll_content = container(content).width(Length::Fill).padding(iced::Padding {
        top: 0.0,
        right: APP_EDITOR_SCROLLBAR_WIDTH + APP_EDITOR_SCROLL_GUTTER,
        bottom: 0.0,
        left: 0.0,
    });

    container(
        container(
            column![
                header,
                scrollable(scroll_content)
                    .direction(Direction::Vertical(
                        Scrollbar::new()
                            .width(APP_EDITOR_SCROLLBAR_WIDTH)
                            .scroller_width(APP_EDITOR_SCROLLBAR_WIDTH),
                    ))
                    .height(Length::Fill),
                footer,
            ]
            .spacing(16)
            .height(Length::Fill),
        )
        .width(Length::Fixed(APP_EDITOR_MODAL_WIDTH))
        .height(Length::Fixed(APP_EDITOR_MODAL_HEIGHT))
        .padding([24, 26])
        .style(modal_card_style),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(iced::alignment::Horizontal::Center)
    .align_y(iced::alignment::Vertical::Center)
    .into()
}

fn build_app_uuid_field<'a>(state: &'a WorkflowState, target_id: &'a str) -> Element<'a, Message> {
    let local_uuid = state
        .apps
        .iter()
        .find(|app| app.id.as_str() == target_id)
        .and_then(|app| app.local_uuid.as_deref());

    let value = local_uuid.unwrap_or("保存到本地数据库后生成 UUID");
    let uuid_panel = container(text(value).size(12))
        .padding([9, 12])
        .width(Length::Fill)
        .style(settings_panel_style);

    let mut uuid_row = row![uuid_panel].spacing(8).align_y(Alignment::Center);

    if let Some(uuid) = local_uuid {
        let copied = state.copied_saved_app_uuid.as_deref() == Some(uuid);
        let copy_content: Element<'_, Message> = if copied {
            text("✓").size(12).line_height(1.0).into()
        } else {
            let copy_icon = svg(assets::get_icon(Icon::Copy))
                .width(Length::Fixed(14.0))
                .height(Length::Fixed(14.0))
                .style(|theme: &Theme, _status| iced::widget::svg::Style {
                    color: Some(theme.palette().text),
                });

            row![copy_icon, text("复制").size(12)].spacing(6).align_y(Alignment::Center).into()
        };

        uuid_row = uuid_row.push(
            button(
                container(copy_content)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(iced::alignment::Horizontal::Center)
                    .align_y(iced::alignment::Vertical::Center),
            )
            .style(rounded_action_btn_style)
            .padding([8, 12])
            .width(Length::Fixed(70.0))
            .height(Length::Fixed(34.0))
            .on_press(Message::WorkflowTool(WorkflowMessage::CopySavedAppUuid(uuid.to_string()))),
        );
    } else {
        uuid_row = uuid_row
            .push(button(text("未生成").size(12)).style(rounded_action_btn_style).padding([8, 12]));
    }

    build_editor_field("应用 UUID", uuid_row.into())
}

fn build_app_organize_field() -> Element<'static, Message> {
    let icon = svg(assets::get_icon(Icon::Grid1x2))
        .width(Length::Fixed(14.0))
        .height(Length::Fixed(14.0))
        .style(|theme: &Theme, _status| iced::widget::svg::Style {
            color: Some(theme.palette().text),
        });

    build_editor_field(
        "应用整理",
        row![
            button(row![icon, text("整理节点位置").size(12)].spacing(6).align_y(Alignment::Center))
                .style(rounded_action_btn_style)
                .padding([8, 12])
                .on_press(Message::WorkflowTool(WorkflowMessage::OrganizeActiveApp)),
        ]
        .spacing(10)
        .align_y(Alignment::Center)
        .into(),
    )
}

#[cfg(test)]
#[path = "app_editor_tests.rs"]
mod app_editor_tests;
