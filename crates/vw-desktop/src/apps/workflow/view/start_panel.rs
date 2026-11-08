//! 工作流起始节点面板模块，负责渲染起始输入变量列表、编辑弹窗和内置变量区。

use super::*;
use crate::app::components::system_settings_common::settings_hover_help_label;
use iced::widget::{column, row};

/// 构建 start visual section 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_start_visual_section<'a>(
    i18n: crate::app::i18n::I18n,
    state: &'a WorkflowState,
    variables: &'a [super::state::WorkflowStartVariableDraft],
    validation: &'a super::state::WorkflowNodeEditorValidation,
) -> Element<'a, Message> {
    let hovered_index =
        state.node_editor.as_ref().and_then(|editor| editor.hovered_start_variable_index);

    column![
        build_start_variable_section(i18n, variables, hovered_index, validation),
        build_start_builtin_variables_section(i18n, state),
    ]
    .spacing(12)
    .into()
}

/// 构建 start variable editor modal 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_start_variable_editor_modal(
    i18n: crate::app::i18n::I18n,
    state: &WorkflowState,
) -> Element<'_, Message> {
    let Some(start_variable_editor) =
        state.node_editor.as_ref().and_then(|editor| editor.start_variable_editor.as_ref())
    else {
        return container(Space::new().width(1).height(1)).into();
    };

    let title = match start_variable_editor.mode {
        super::state::WorkflowStartVariableEditorMode::Create => i18n.__("添加变量"),
        super::state::WorkflowStartVariableEditorMode::Edit(_) => i18n.__("编辑变量"),
    };

    let close = Message::WorkflowTool(WorkflowMessage::NodeEditorStartCloseVariableEditor);
    let header = settings_modal_header(text(title).size(20), settings_close_button(close.clone()));
    let actions = row![
        Space::new().width(Length::Fill),
        button(text(i18n.__("取消")))
            .style(rounded_action_btn_style)
            .padding([9, 14])
            .on_press(close),
        button(text(i18n.__("保存")))
            .style(primary_action_btn_style)
            .padding([9, 14])
            .on_press(Message::WorkflowTool(WorkflowMessage::NodeEditorStartSubmitVariableEditor,)),
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    let card = settings_modal_layout(
        header,
        build_start_variable_card(i18n, start_variable_editor),
        Some(actions.into()),
    )
    .width(Length::Fill)
    .max_width(540.0)
    .max_height(660.0);

    settings_modal_overlay(None, Message::None, container(card).padding(12))
}

/// 构建 start variable section 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_start_variable_section<'a>(
    i18n: crate::app::i18n::I18n,
    variables: &'a [super::state::WorkflowStartVariableDraft],
    hovered_index: Option<usize>,
    validation: &'a super::state::WorkflowNodeEditorValidation,
) -> Element<'a, Message> {
    let mut variable_list = column![].spacing(8).width(Length::Fill);

    if variables.is_empty() {
        variable_list = variable_list.push(
            container(
                text(i18n.__("当前没有输入变量。新增后会写入 start.variables。"))
                    .size(12)
                    .style(settings_muted_text_style),
            )
            .padding([12, 14])
            .style(value_card_style),
        );
    } else {
        for (index, variable) in variables.iter().enumerate() {
            variable_list = variable_list.push(build_start_variable_summary_item(
                i18n,
                index,
                hovered_index,
                variable,
                validation,
            ));
        }
    }

    column![
        row![
            column![text(i18n.__("变量")).size(16)].spacing(4),
            Space::new().width(Length::Fill),
            button(text(i18n.__("新增变量")).size(11))
                .style(primary_action_btn_style)
                .padding([6, 10])
                .on_press(Message::WorkflowTool(WorkflowMessage::NodeEditorStartAddVariable,)),
        ]
        .align_y(Alignment::Center),
        variable_list,
    ]
    .spacing(12)
    .into()
}

/// 构建 start variable summary item 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_start_variable_summary_item<'a>(
    i18n: crate::app::i18n::I18n,
    index: usize,
    _hovered_index: Option<usize>,
    variable: &'a super::state::WorkflowStartVariableDraft,
    validation: &'a super::state::WorkflowNodeEditorValidation,
) -> Element<'a, Message> {
    let name = if variable.variable.trim().is_empty() {
        i18n.format("变量 %s", &[(index + 1).to_string().into()])
            .expect("valid workflow display template")
    } else {
        variable.variable.clone()
    };
    let label = variable.label.trim();
    let summary_error = start_variable_summary_error(i18n, index, validation);

    let label_fragment: Element<'a, Message> = if label.is_empty() {
        container(Space::new().width(1).height(1)).into()
    } else {
        row![
            text("·").size(12).style(settings_muted_text_style),
            text(label).size(13).style(settings_muted_text_style),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .into()
    };
    let action_buttons: Element<'a, Message> = row![
        build_start_variable_action_button(
            Icon::Pencil,
            false,
            Message::WorkflowTool(WorkflowMessage::NodeEditorStartSelectVariable(index)),
        ),
        build_start_variable_action_button(
            Icon::Trash,
            true,
            Message::WorkflowTool(WorkflowMessage::NodeEditorStartRemoveVariable(index)),
        ),
    ]
    .spacing(6)
    .into();

    let row_content = row![
        row![text(name).size(13), label_fragment]
            .spacing(6)
            .align_y(Alignment::Center)
            .width(Length::Fill),
        start_variable_badge(start_variable_value_type_label(&variable.input_type)),
        action_buttons,
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let content: Element<'a, Message> = if let Some(error) = summary_error {
        column![row_content, text(error).size(12).color(Color::from_rgba8(0xB4, 0x23, 0x18, 1.0)),]
            .spacing(6)
            .into()
    } else {
        row_content.into()
    };

    mouse_area(container(content).width(Length::Fill).padding([10, 12]).style(value_card_style))
        .on_enter(Message::WorkflowTool(WorkflowMessage::NodeEditorStartVariableHovered(Some(
            index,
        ))))
        .on_exit(Message::WorkflowTool(WorkflowMessage::NodeEditorStartVariableHovered(None)))
        .into()
}

fn build_start_variable_action_button(
    icon: Icon,
    danger: bool,
    message: Message,
) -> Element<'static, Message> {
    button(
        svg(assets::get_icon(icon)).width(Length::Fixed(12.0)).height(Length::Fixed(12.0)).style(
            move |theme: &Theme, _status| iced::widget::svg::Style {
                color: Some(if danger {
                    Color::from_rgba8(
                        0xDC,
                        0x45,
                        0x45,
                        if is_dark_theme(theme) { 0.92 } else { 0.84 },
                    )
                } else {
                    theme.palette().text.scale_alpha(if is_dark_theme(theme) { 0.76 } else { 0.70 })
                }),
            },
        ),
    )
    .padding(6)
    .style(round_icon_btn_style)
    .on_press(message)
    .into()
}

/// 构建 start builtin variables section 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_start_builtin_variables_section(
    i18n: crate::app::i18n::I18n,
    state: &WorkflowState,
) -> Element<'_, Message> {
    let mut list = column![].spacing(8);

    for item in start_builtin_variables(i18n, state) {
        list = list.push(build_start_builtin_variable_item(i18n, item));
    }

    column![text(i18n.__("内置变量")).size(16), list].spacing(12).into()
}

/// 构建 start builtin variable item 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_start_builtin_variable_item(
    i18n: crate::app::i18n::I18n,
    item: StartBuiltinVariableItem,
) -> Element<'static, Message> {
    let mut label = row![settings_hover_help_label(
        i18n,
        None,
        item.name,
        text(item.name).size(13),
        item.description,
    )]
    .spacing(6)
    .align_y(Alignment::Center);
    if item.legacy {
        label = label.push(start_variable_badge(i18n.__("兼容字段")));
    }
    container(
        row![container(label.wrap()).width(Length::Fill), start_variable_badge(item.value_type),]
            .spacing(12)
            .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding([12, 14])
    .into()
}

#[cfg(test)]
#[path = "start_panel_tests.rs"]
mod start_panel_tests;
