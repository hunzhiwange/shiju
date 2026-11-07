//! 工作流变量视图模块，负责变量面板、变量编辑器、变量卡片和校验摘要的渲染。

use super::*;
use iced::widget::{column, row};

/// 构建 variable panel modal 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_variable_panel_modal(
    i18n: crate::app::i18n::I18n,
    state: &WorkflowState,
) -> Element<'_, Message> {
    let Some(panel) = state.variable_panel.as_ref() else {
        return container(Space::new().width(1).height(1)).into();
    };

    let (title, subtitle, add_button, body): (
        &str,
        &str,
        Option<Element<'_, Message>>,
        Element<'_, Message>,
    ) = match panel {
        WorkflowVariablePanelKind::System => (
            i18n.__("系统变量"),
            i18n.__("这些变量由运行时自动注入，只读展示，方便和 Dify 的全局变量保持一致。"),
            None,
            build_system_variable_list(i18n, state),
        ),
        WorkflowVariablePanelKind::Environment => (
            i18n.__("环境变量"),
            i18n.__("环境变量会随 yml 一起保存，适合放 API Key、URL、开关等全局配置。"),
            Some(
                button(text(i18n.__("新增环境变量")))
                    .style(primary_action_btn_style)
                    .padding([9, 14])
                    .on_press(Message::WorkflowTool(
                        WorkflowMessage::OpenCreateEnvironmentVariableEditor,
                    ))
                    .into(),
            ),
            build_environment_variable_list(i18n, state),
        ),
        WorkflowVariablePanelKind::Conversation => (
            i18n.__("会话变量"),
            i18n.__("会话变量会跟随调试会话流转，适合沉淀结构化上下文与中间状态。"),
            Some(
                button(text(i18n.__("新增会话变量")))
                    .style(primary_action_btn_style)
                    .padding([9, 14])
                    .on_press(Message::WorkflowTool(
                        WorkflowMessage::OpenCreateConversationVariableEditor,
                    ))
                    .into(),
            ),
            build_conversation_variable_list(i18n, state),
        ),
    };

    let mut header_row = row![
        column![text(title).size(24), text(subtitle).size(12).style(settings_muted_text_style),]
            .spacing(4),
        Space::new().width(Length::Fill),
    ]
    .align_y(Alignment::Center)
    .spacing(8);

    if let Some(add_button) = add_button {
        header_row = header_row.push(add_button);
    }

    header_row = header_row
        .push(settings_close_button(Message::WorkflowTool(WorkflowMessage::CloseVariablePanel)));

    container(
        container(column![header_row, body,].spacing(14))
            .width(Length::Fixed(780.0))
            .padding([20, 22])
            .style(modal_card_style),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(iced::alignment::Horizontal::Center)
    .align_y(iced::alignment::Vertical::Center)
    .style(modal_backdrop_style)
    .into()
}

/// 构建 variable editor modal 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_variable_editor_modal(
    i18n: crate::app::i18n::I18n,
    state: &WorkflowState,
) -> Element<'_, Message> {
    let Some(editor) = state.variable_editor.as_ref() else {
        return container(Space::new().width(1).height(1)).into();
    };

    let (title, subtitle, type_hint) = match &editor.mode {
        super::state::WorkflowVariableEditorMode::CreateEnvironment => (
            i18n.__("新增环境变量"),
            i18n.__("环境变量推荐使用 string / number / secret 三种类型。"),
            "string / number / secret",
        ),
        super::state::WorkflowVariableEditorMode::EditEnvironment(_) => (
            i18n.__("编辑环境变量"),
            i18n.__("保留原始 raw 字段，只覆盖常用的 name / description / value_type / value。"),
            "string / number / secret",
        ),
        super::state::WorkflowVariableEditorMode::CreateConversation => (
            i18n.__("新增会话变量"),
            i18n.__("会话变量类型可以更宽，支持 string / number / object / array[string] 等 Dify 风格值。"),
            "string / number / object / array[string]",
        ),
        super::state::WorkflowVariableEditorMode::EditConversation(_) => (
            i18n.__("编辑会话变量"),
            i18n.__("会话变量 value 直接使用 YAML 编辑，方便录入对象或数组。"),
            "string / number / object / array[string]",
        ),
    };

    container(
        container(
            column![
                row![
                    column![
                        text(title).size(24),
                        text(subtitle).size(12).style(settings_muted_text_style),
                    ]
                    .spacing(4),
                    Space::new().width(Length::Fill),
                    settings_close_button(Message::WorkflowTool(
                        WorkflowMessage::CloseVariableEditor
                    )),
                ]
                .align_y(Alignment::Center),
                build_editor_field(
                    i18n.__("变量名称"),
                    workflow_text_input(
                        i18n.__("例如：api_key / conversation_state"),
                        &editor.name,
                        |value| {
                            Message::WorkflowTool(WorkflowMessage::VariableEditorNameChanged(value))
                        }
                    ),
                ),
                build_editor_field(
                    i18n.__("变量类型"),
                    workflow_text_input(type_hint, &editor.value_type, |value| {
                        Message::WorkflowTool(WorkflowMessage::VariableEditorTypeChanged(value))
                    }),
                ),
                build_editor_field(
                    i18n.__("变量描述"),
                    workflow_text_input(
                        i18n.__("描述这个变量在工作流中的用途"),
                        &editor.description,
                        |value| {
                            Message::WorkflowTool(
                                WorkflowMessage::VariableEditorDescriptionChanged(value),
                            )
                        }
                    ),
                ),
                build_editor_field(
                    i18n.__("变量值 YAML"),
                    text_editor(&editor.raw_value_editor)
                        .placeholder(i18n.__("输入变量值，支持字符串、数字、对象或数组 YAML"))
                        .on_action(|action| {
                            Message::WorkflowTool(WorkflowMessage::VariableEditorValueAction(
                                action,
                            ))
                        })
                        .padding([10, 12])
                        .height(Length::Fixed(220.0))
                        .style(editor_style)
                        .into(),
                ),
                row![
                    Space::new().width(Length::Fill),
                    button(text(i18n.__("取消")))
                        .style(rounded_action_btn_style)
                        .padding([9, 14])
                        .on_press(Message::WorkflowTool(WorkflowMessage::CloseVariableEditor)),
                    button(text(i18n.__("保存变量")))
                        .style(primary_action_btn_style)
                        .padding([9, 14])
                        .on_press(Message::WorkflowTool(WorkflowMessage::SubmitVariableEditor)),
                ]
                .spacing(10),
            ]
            .spacing(14),
        )
        .width(Length::Fixed(740.0))
        .padding([20, 22])
        .style(modal_card_style),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(iced::alignment::Horizontal::Center)
    .align_y(iced::alignment::Vertical::Center)
    .style(modal_backdrop_style)
    .into()
}

/// 构建 system variable list 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_system_variable_list(
    i18n: crate::app::i18n::I18n,
    state: &WorkflowState,
) -> Element<'_, Message> {
    let items = state.active_meta().map(workflow_system_variables).unwrap_or_default();

    if items.is_empty() {
        return container(text(i18n.__("当前没有可显示的系统变量")).size(13))
            .padding(12)
            .style(value_card_style)
            .into();
    }

    let mut list = column![].spacing(10);
    for item in items {
        list = list.push(variable_card(
            i18n,
            item.name.to_string(),
            item.value_type.to_string(),
            i18n.__(item.description).to_string(),
            i18n.__("运行时注入").to_string(),
            None,
            None,
        ));
    }

    scrollable(container(list).padding([0, 7]))
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
        .height(Length::Fixed(420.0))
        .into()
}

/// 构建 environment variable list 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_environment_variable_list(
    i18n: crate::app::i18n::I18n,
    state: &WorkflowState,
) -> Element<'_, Message> {
    if state.environment_variables.is_empty() {
        return container(text(i18n.__("还没有环境变量")).size(13))
            .padding(12)
            .style(value_card_style)
            .into();
    }

    let mut list = column![].spacing(10);
    for variable in &state.environment_variables {
        let description = if variable.description.trim().is_empty() {
            i18n.__("无描述").to_string()
        } else {
            variable.description.clone()
        };
        let preview = variable_value_preview(&variable.value, Some(&variable.value_type));
        list = list.push(variable_card(
            i18n,
            variable.name.clone(),
            variable.value_type.clone(),
            description,
            preview,
            Some(Message::WorkflowTool(WorkflowMessage::OpenEditEnvironmentVariableEditor(
                variable.id.clone(),
            ))),
            Some(Message::WorkflowTool(WorkflowMessage::DeleteEnvironmentVariable(
                variable.id.clone(),
            ))),
        ));
    }

    scrollable(container(list).padding([0, 7]))
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
        .height(Length::Fixed(420.0))
        .into()
}

/// 构建 conversation variable list 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_conversation_variable_list(
    i18n: crate::app::i18n::I18n,
    state: &WorkflowState,
) -> Element<'_, Message> {
    if state.conversation_variables.is_empty() {
        return container(text(i18n.__("还没有会话变量")).size(13))
            .padding(12)
            .style(value_card_style)
            .into();
    }

    let mut list = column![].spacing(10);
    for variable in &state.conversation_variables {
        let description = if variable.description.trim().is_empty() {
            i18n.__("无描述").to_string()
        } else {
            variable.description.clone()
        };
        let preview = variable_value_preview(&variable.value, None);
        list = list.push(variable_card(
            i18n,
            variable.name.clone(),
            variable.value_type.clone(),
            description,
            preview,
            Some(Message::WorkflowTool(WorkflowMessage::OpenEditConversationVariableEditor(
                variable.id.clone(),
            ))),
            Some(Message::WorkflowTool(WorkflowMessage::DeleteConversationVariable(
                variable.id.clone(),
            ))),
        ));
    }

    scrollable(container(list).padding([0, 7]))
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
        .height(Length::Fixed(420.0))
        .into()
}

/// 提供 variable card 功能。
///
/// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
pub(super) fn variable_card(
    i18n: crate::app::i18n::I18n,
    name: String,
    value_type: String,
    description: String,
    preview: String,
    edit_message: Option<Message>,
    delete_message: Option<Message>,
) -> Element<'static, Message> {
    let mut actions = row![Space::new().width(Length::Fill)].spacing(8).align_y(Alignment::Center);

    if let Some(edit_message) = edit_message {
        actions = actions.push(
            button(text(i18n.__("编辑")).size(12))
                .style(rounded_action_btn_style)
                .padding([6, 10])
                .on_press(edit_message),
        );
    }
    if let Some(delete_message) = delete_message {
        actions = actions.push(
            button(text(i18n.__("删除")).size(12))
                .style(danger_action_btn_style)
                .padding([6, 10])
                .on_press(delete_message),
        );
    }

    container(
        column![
            row![
                text(name).size(15),
                Space::new().width(Length::Fill),
                container(text(value_type).size(11)).padding([4, 8]).style(value_card_style),
            ]
            .align_y(Alignment::Center),
            text(description).size(12).color(Color::from_rgba8(0x11, 0x18, 0x27, 0.64)),
            container(text(preview).size(12)).padding([8, 10]).style(value_card_style),
            actions,
        ]
        .spacing(8),
    )
    .padding(12)
    .style(inspector_style)
    .into()
}

/// 提供 variable value preview 功能。
///
/// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
pub(super) fn variable_value_preview(
    value: &serde_yaml::Value,
    value_type: Option<&str>,
) -> String {
    if value_type.is_some_and(|kind| kind.eq_ignore_ascii_case("secret")) {
        return "******".to_string();
    }

    let yaml = serde_yaml::to_string(value).unwrap_or_else(|_| "<invalid>".to_string());
    let preview = yaml.trim().replace('\n', " ");
    let preview_chars = preview.chars().collect::<Vec<_>>();
    if preview_chars.len() > 120 {
        format!("{}...", preview_chars.into_iter().take(120).collect::<String>())
    } else if preview.is_empty() {
        "<empty>".to_string()
    } else {
        preview
    }
}

/// 构建 editor field 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_editor_field<'a>(
    label: &'a str,
    control: Element<'a, Message>,
) -> Element<'a, Message> {
    build_editor_field_validated(label, control, None)
}

/// 构建 editor field validated 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_editor_field_validated<'a>(
    label: &'a str,
    control: Element<'a, Message>,
    error: Option<String>,
) -> Element<'a, Message> {
    let mut content =
        column![text(label).size(12).style(settings_muted_text_style), control].spacing(6);

    if let Some(error) = error {
        content = content.push(text(error).size(12).style(|theme: &Theme| {
            iced::widget::text::Style { color: Some(theme.extended_palette().danger.base.color) }
        }));
    }

    content.into()
}

/// 构建 node validation summary 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_node_validation_summary(
    i18n: crate::app::i18n::I18n,
    validation: &super::state::WorkflowNodeEditorValidation,
) -> Element<'_, Message> {
    let mut items = column![
        text(i18n.__("当前节点还有未修正的字段")).size(14),
        text(i18n.__("请先处理下面这些错误，再保存节点。")).size(12),
    ]
    .spacing(6);

    for error in validation.field_errors.iter().take(8) {
        items = items.push(text(format!("- {}", error.message.render(i18n))).size(12));
    }

    if validation.field_errors.len() > 8 {
        items = items.push(
            text(
                i18n.format(
                    "还有 %s 项未展示",
                    &[(validation.field_errors.len() - 8).to_string().into()],
                )
                .expect("valid workflow display template"),
            )
            .size(12),
        );
    }

    container(items).padding([12, 14]).style(validation_summary_style).into()
}

/// 构建 inline error 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_inline_error(message: impl Into<String>) -> Element<'static, Message> {
    text(message.into())
        .size(12)
        .style(|theme: &Theme| iced::widget::text::Style {
            color: Some(theme.extended_palette().danger.base.color),
        })
        .into()
}

#[cfg(test)]
#[path = "variables_tests.rs"]
mod variables_tests;
