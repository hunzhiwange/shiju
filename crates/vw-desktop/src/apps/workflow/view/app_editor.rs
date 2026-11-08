//! 工作流应用编辑器视图，负责应用名称、图标、描述和请求限制的表单渲染。

use super::*;
use crate::app::components::system_settings_common::{
    settings_field_label, settings_group_card, settings_group_heading_with_help,
    settings_hover_help_label, settings_toggle,
};
use iced::widget::{column, row};

/// 构建或更新 build app editor modal 相关行为。
///
/// 参数由调用方提供，返回值直接交给上层视图或状态处理；本函数不执行额外错误恢复。
pub(super) fn build_app_editor_modal(
    i18n: crate::app::i18n::I18n,
    state: &WorkflowState,
) -> Element<'_, Message> {
    let Some(editor) = state.app_editor.as_ref() else {
        return container(Space::new().width(1).height(1)).into();
    };

    const APP_EDITOR_MODAL_WIDTH: f32 = 680.0;
    const APP_EDITOR_MODAL_HEIGHT: f32 = 620.0;

    let title = match &editor.mode {
        super::state::WorkflowAppEditorMode::Create => i18n.__("新增应用"),
        super::state::WorkflowAppEditorMode::Edit(_) => i18n.__("编辑应用信息"),
    };

    let subtitle = match &editor.mode {
        super::state::WorkflowAppEditorMode::Create => {
            i18n.__("创建一个新的 workflow 应用，后续可以单独保存成 yml 文件。")
        }
        super::state::WorkflowAppEditorMode::Edit(_) => {
            i18n.__("修改应用元数据，不会丢失原始 Dify yml 里的节点业务字段。")
        }
    };

    let close = Message::WorkflowTool(WorkflowMessage::CloseAppEditor);
    let header = settings_modal_header(
        settings_hover_help_label(i18n, None, title, text(title).size(20), subtitle).wrap(),
        settings_close_button(close.clone()),
    );

    let mut fields = column![].spacing(14);

    if let super::state::WorkflowAppEditorMode::Edit(target_id) = &editor.mode {
        fields = fields.push(build_app_uuid_field(i18n, state, target_id));
        if state.active_app_id.as_deref() == Some(target_id.as_str()) {
            fields = fields.push(build_app_organize_field(i18n));
        }
    }

    let fields = fields
        .push(build_editor_field(
            i18n.__("应用名称"),
            workflow_text_input(i18n.__("例如：客服分流工作流"), &editor.name, |value| {
                Message::WorkflowTool(WorkflowMessage::AppEditorNameChanged(value))
            }),
        ))
        .push(build_editor_field(
            i18n.__("应用图标"),
            workflow_text_input(i18n.__("例如：🤖"), &editor.icon, |value| {
                Message::WorkflowTool(WorkflowMessage::AppEditorIconChanged(value))
            }),
        ))
        .push(build_editor_field(
            i18n.__("应用描述"),
            workflow_text_input(
                i18n.__("描述这个 workflow 用来做什么"),
                &editor.description,
                |value| Message::WorkflowTool(WorkflowMessage::AppEditorDescriptionChanged(value)),
            ),
        ))
        .push({
            let label = i18n.__("使用 web app 图标替换 🤖");
            column![
                settings_hover_help_label(
                    i18n,
                    None,
                    label,
                    settings_field_label(label),
                    i18n.__("在分享和 Explore 场景里用应用图标替换默认机器人图标"),
                )
                .wrap(),
                settings_toggle(editor.use_icon_as_answer_icon).on_toggle(|value| {
                    Message::WorkflowTool(WorkflowMessage::AppEditorUseIconAsAnswerIconChanged(
                        value,
                    ))
                }),
            ]
            .spacing(6)
        })
        .push(build_editor_field(
            i18n.__("最大活跃请求数"),
            workflow_text_input(
                i18n.__("0 表示不限制"),
                &editor.max_active_requests_input,
                |value| {
                    Message::WorkflowTool(WorkflowMessage::AppEditorMaxActiveRequestsChanged(value))
                },
            ),
        ));

    let footer = row![
        Space::new().width(Length::Fill),
        button(text(i18n.__("取消")))
            .style(rounded_action_btn_style)
            .padding([9, 14])
            .on_press(close.clone()),
        button(text(i18n.__("保存应用信息")))
            .style(primary_action_btn_style)
            .padding([9, 14])
            .on_press(Message::WorkflowTool(WorkflowMessage::SubmitAppEditor)),
    ]
    .spacing(10);

    let body = column![
        settings_group_heading_with_help(
            i18n,
            None,
            i18n.__("应用基础信息"),
            i18n.__("表单结构参考 Dify 应用设置页，先整理名称、图标和描述。"),
        ),
        settings_group_card(fields).padding(14),
    ]
    .spacing(12);
    let card = settings_modal_layout(header, body, Some(footer.into()))
        .width(APP_EDITOR_MODAL_WIDTH)
        .height(APP_EDITOR_MODAL_HEIGHT);

    settings_modal_overlay(None, close, container(card).padding(12))
}

fn build_app_uuid_field<'a>(
    i18n: crate::app::i18n::I18n,
    state: &'a WorkflowState,
    target_id: &'a str,
) -> Element<'a, Message> {
    let local_uuid = state
        .apps
        .iter()
        .find(|app| app.id.as_str() == target_id)
        .and_then(|app| app.local_uuid.as_deref());

    let value = local_uuid.unwrap_or(i18n.__("保存到本地数据库后生成 UUID"));
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

            row![copy_icon, text(i18n.__("复制")).size(12)]
                .spacing(6)
                .align_y(Alignment::Center)
                .into()
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
        uuid_row = uuid_row.push(
            button(text(i18n.__("未生成")).size(12))
                .style(rounded_action_btn_style)
                .padding([8, 12]),
        );
    }

    build_editor_field(i18n.__("应用 UUID"), uuid_row.into())
}

fn build_app_organize_field(i18n: crate::app::i18n::I18n) -> Element<'static, Message> {
    let icon = svg(assets::get_icon(Icon::Grid1x2))
        .width(Length::Fixed(14.0))
        .height(Length::Fixed(14.0))
        .style(|theme: &Theme, _status| iced::widget::svg::Style {
            color: Some(theme.palette().text),
        });

    build_editor_field(
        i18n.__("应用整理"),
        row![
            button(
                row![icon, text(i18n.__("整理节点位置")).size(12)]
                    .spacing(6)
                    .align_y(Alignment::Center)
            )
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

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "app_editor_ui_refresh_38_tests.rs"]
mod ui_refresh_38_tests;
