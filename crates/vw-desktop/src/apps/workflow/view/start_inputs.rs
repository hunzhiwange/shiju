//! 工作流起始节点输入配置视图模块，负责渲染变量默认值、选项和文件上传设置。

use super::*;
use iced::widget::{column, row};

/// 提供 start variable file type options 功能。
///
/// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
pub(super) fn start_variable_file_type_options() -> [(&'static str, &'static str, &'static str); 5]
{
    [
        ("document", "文档", "pdf, doc, docx, txt, md"),
        ("image", "图片", "png, jpg, jpeg, webp, gif"),
        ("audio", "音频", "mp3, wav, m4a"),
        ("video", "视频", "mp4, mov, avi"),
        ("custom", "自定义", "手动填写扩展名"),
    ]
}

/// 构建 start variable option editor 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_start_variable_option_editor<'a>(
    variable: &'a super::state::WorkflowStartVariableDraft,
) -> Element<'a, Message> {
    let mut option_list = column![].spacing(8);

    if variable.options.is_empty() {
        option_list = option_list.push(
            container(
                text("当前还没有选项，至少新增一个后才能保存。")
                    .size(12)
                    .style(settings_muted_text_style),
            )
            .padding([10, 12])
            .style(value_card_style),
        );
    } else {
        for (index, option) in variable.options.iter().enumerate() {
            option_list = option_list.push(
                row![
                    workflow_text_input("选项内容", option, move |value| {
                        Message::WorkflowTool(
                            WorkflowMessage::NodeEditorStartVariableEditorOptionChanged(
                                index, value,
                            ),
                        )
                    }),
                    button(text("删除").size(12))
                        .style(danger_action_btn_style)
                        .padding([8, 10])
                        .on_press(Message::WorkflowTool(
                            WorkflowMessage::NodeEditorStartVariableEditorRemoveOption(index),
                        )),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }
    }

    build_editor_field(
        "下拉选项",
        container(
            column![
                option_list,
                button(text("新增选项").size(12))
                    .style(rounded_action_btn_style)
                    .padding([8, 12])
                    .on_press(Message::WorkflowTool(
                        WorkflowMessage::NodeEditorStartVariableEditorAddOption,
                    )),
            ]
            .spacing(8),
        )
        .into(),
    )
}

/// 构建 start variable select default field 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_start_variable_select_default_field<'a>(
    variable: &'a super::state::WorkflowStartVariableDraft,
) -> Element<'a, Message> {
    let options = variable
        .options
        .iter()
        .map(|option| option.trim())
        .filter(|option| !option.is_empty())
        .map(|option| option.to_string())
        .collect::<Vec<_>>();

    if options.is_empty() {
        return build_editor_field(
            "默认值",
            container(text("请先添加至少一个下拉选项。").size(12).style(settings_muted_text_style))
                .padding([10, 12])
                .style(value_card_style)
                .into(),
        );
    }

    let selected =
        options.iter().find(|option| option.as_str() == variable.default_value.trim()).cloned();
    let select_default_picker: Element<'a, Message> = pick_list(options, selected, |value| {
        Message::WorkflowTool(WorkflowMessage::NodeEditorStartVariableEditorDefaultChanged(value))
    })
    .padding([10, 12])
    .text_size(13)
    .style(settings_pick_list_style)
    .menu_style(settings_pick_list_menu_style)
    .width(Length::Fill)
    .into();

    build_editor_field(
        "默认值",
        row![
            select_default_picker,
            button(text("清空").size(12))
                .style(rounded_action_btn_style)
                .padding([8, 12])
                .on_press(Message::WorkflowTool(
                    WorkflowMessage::NodeEditorStartVariableEditorDefaultChanged(String::new()),
                )),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .into(),
    )
}

/// 构建 start variable file type button 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_start_variable_file_type_button(
    value: &'static str,
    label: &'static str,
    description: &'static str,
    selected: bool,
) -> Element<'static, Message> {
    button(
        column![text(label).size(12), text(description).size(11).style(settings_muted_text_style),]
            .spacing(4)
            .width(Length::Fill),
    )
    .style(if selected { primary_action_btn_style } else { rounded_action_btn_style })
    .padding([10, 12])
    .width(Length::Fixed(116.0))
    .on_press(Message::WorkflowTool(WorkflowMessage::NodeEditorStartVariableEditorToggleFileType(
        value.to_string(),
    )))
    .into()
}

/// 构建 start variable upload method button 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_start_variable_upload_method_button(
    value: &'static str,
    label: &'static str,
    selected: bool,
) -> Element<'static, Message> {
    button(text(label).size(12))
        .style(if selected { primary_action_btn_style } else { rounded_action_btn_style })
        .padding([10, 12])
        .width(Length::Fill)
        .on_press(Message::WorkflowTool(
            WorkflowMessage::NodeEditorStartVariableEditorUploadMethodChanged(value.to_string()),
        ))
        .into()
}

fn build_start_variable_default_file_button(
    icon: Icon,
    label: &'static str,
    message: WorkflowMessage,
) -> Element<'static, Message> {
    button(
        row![
            svg(assets::get_icon(icon)).width(Length::Fixed(14.0)).height(Length::Fixed(14.0)),
            text(label).size(13),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .style(rounded_action_btn_style)
    .padding([10, 12])
    .width(Length::Fill)
    .on_press(Message::WorkflowTool(message))
    .into()
}

fn build_start_variable_file_default_value<'a>(
    start_variable_editor: &'a super::state::WorkflowStartVariableEditorDraft,
) -> Element<'a, Message> {
    let variable = &start_variable_editor.variable;
    let selected_default_count = variable.default_file_values.len();
    let max_default_count = if variable.input_type == "file-list" {
        usize::from(super::state::normalized_start_variable_file_list_max_length(
            &variable.max_length_input,
        ))
    } else {
        1
    };
    let supports_local =
        variable.allowed_file_upload_methods.iter().any(|item| item == "local_file");
    let supports_url = variable.allowed_file_upload_methods.iter().any(|item| item == "remote_url");

    let action_row: Element<'a, Message> = match (supports_local, supports_url) {
        (true, true) => row![
            build_start_variable_default_file_button(
                Icon::CloudUpload,
                "从本地上传",
                WorkflowMessage::NodeEditorStartVariableEditorPickDefaultFile,
            ),
            build_start_variable_default_file_button(
                Icon::Link,
                "粘贴文件链接",
                WorkflowMessage::NodeEditorStartVariableEditorOpenDefaultFileUrlInput,
            ),
        ]
        .spacing(8)
        .into(),
        (true, false) => build_start_variable_default_file_button(
            Icon::CloudUpload,
            "从本地上传",
            WorkflowMessage::NodeEditorStartVariableEditorPickDefaultFile,
        ),
        (false, true) => build_start_variable_default_file_button(
            Icon::Link,
            "粘贴文件链接",
            WorkflowMessage::NodeEditorStartVariableEditorOpenDefaultFileUrlInput,
        ),
        (false, false) => {
            container(text("请先选择至少一种上传方式。").size(12).style(settings_muted_text_style))
                .padding([10, 12])
                .style(value_card_style)
                .into()
        }
    };

    let url_input_overlay: Element<'a, Message> =
        if start_variable_editor.show_default_file_url_input {
            container(
                column![
                    row![
                        workflow_text_input(
                            "输入文件链接",
                            &start_variable_editor.default_file_url_input,
                            |value| Message::WorkflowTool(
                                WorkflowMessage::NodeEditorStartVariableEditorDefaultFileUrlChanged(
                                    value
                                ),
                            ),
                        ),
                        button(text("好的").size(12))
                            .style(primary_action_btn_style)
                            .padding([10, 12])
                            .on_press(Message::WorkflowTool(
                                WorkflowMessage::NodeEditorStartVariableEditorSubmitDefaultFileUrl,
                            )),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                    button(text("取消").size(12))
                        .style(rounded_action_btn_style)
                        .padding([8, 10])
                        .on_press(Message::WorkflowTool(
                            WorkflowMessage::NodeEditorStartVariableEditorCloseDefaultFileUrlInput,
                        )),
                ]
                .spacing(10),
            )
            .padding([12, 14])
            .width(Length::Fixed(330.0))
            .style(floating_panel_style)
            .into()
        } else {
            container(Space::new().width(1).height(1)).into()
        };

    let current_value_preview: Element<'a, Message> = if variable.default_file_values.is_empty() {
        container(Space::new().width(1).height(1)).into()
    } else {
        container(
            column![
                text(if variable.input_type == "file-list" {
                    format!("当前默认值 {selected_default_count}/{max_default_count}")
                } else {
                    "当前默认值".to_string()
                })
                .size(12)
                .style(settings_muted_text_style),
                column(
                    variable
                        .default_file_values
                        .iter()
                        .enumerate()
                        .map(|(index, item)| {
                            row![
                                text(item.as_str()).size(12).width(Length::Fill),
                                button(
                                    svg(assets::get_icon(Icon::Trash))
                                        .width(Length::Fixed(12.0))
                                        .height(Length::Fixed(12.0)),
                                )
                                .padding(6)
                                .style(round_icon_btn_style)
                                .on_press(Message::WorkflowTool(
                                    WorkflowMessage::NodeEditorStartVariableEditorRemoveDefaultFile(
                                        index
                                    ),
                                )),
                            ]
                            .spacing(8)
                            .align_y(Alignment::Center)
                            .into()
                        })
                        .collect::<Vec<Element<'a, Message>>>(),
                )
                .spacing(8),
            ]
            .spacing(6),
        )
        .padding([10, 12])
        .style(value_card_style)
        .into()
    };

    column![action_row, url_input_overlay, current_value_preview].spacing(8).into()
}

/// 构建 start variable file settings 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_start_variable_file_settings<'a>(
    start_variable_editor: &'a super::state::WorkflowStartVariableEditorDraft,
) -> Element<'a, Message> {
    let variable = &start_variable_editor.variable;
    let mut file_type_buttons = row![].spacing(8);
    for (value, label, description) in start_variable_file_type_options() {
        let selected = variable.allowed_file_types.iter().any(|item| item == value);
        file_type_buttons = file_type_buttons.push(build_start_variable_file_type_button(
            value,
            label,
            description,
            selected,
        ));
    }

    let upload_mode =
        if variable.allowed_file_upload_methods.iter().any(|item| item == "local_file")
            && variable.allowed_file_upload_methods.iter().any(|item| item == "remote_url")
        {
            "all"
        } else if variable.allowed_file_upload_methods.iter().any(|item| item == "local_file") {
            "local_file"
        } else if variable.allowed_file_upload_methods.iter().any(|item| item == "remote_url") {
            "remote_url"
        } else {
            ""
        };

    let custom_extensions_field: Element<'a, Message> =
        if variable.allowed_file_types.iter().any(|item| item == "custom") {
            build_editor_field(
                "自定义扩展名",
                workflow_text_input(
                    "例如：.pdf, .csv",
                    &variable.allowed_file_extensions_input,
                    |value| {
                        Message::WorkflowTool(
                            WorkflowMessage::NodeEditorStartVariableEditorExtensionsChanged(value),
                        )
                    },
                ),
            )
        } else {
            container(Space::new().width(1).height(1)).into()
        };

    let max_upload_count_field: Element<'a, Message> = if variable.input_type == "file-list" {
        let current_value = super::state::normalized_start_variable_file_list_max_length(
            &variable.max_length_input,
        );
        build_editor_field(
            "最大上传数",
            column![
                text("文档 <15.00 MB, 图片 <10.00 MB, 音频 <50.00 MB, 视频 <100.00 MB")
                    .size(12)
                    .style(settings_muted_text_style),
                row![
                    container(text(current_value.to_string()).size(16))
                        .padding([10, 12])
                        .width(Length::Fixed(52.0))
                        .style(value_card_style),
                    slider(1.0..=10.0, current_value as f32, |value: f32| {
                        Message::WorkflowTool(
                            WorkflowMessage::NodeEditorStartVariableEditorMaxLengthChanged(
                                (value.round() as u8).clamp(1, 10).to_string(),
                            ),
                        )
                    })
                    .step(1.0_f32)
                    .width(Length::Fill),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            ]
            .spacing(10)
            .into(),
        )
    } else {
        container(Space::new().width(1).height(1)).into()
    };

    let default_value_field: Element<'a, Message> =
        if matches!(variable.input_type.as_str(), "file" | "file-list") {
            build_editor_field(
                "默认值",
                build_start_variable_file_default_value(start_variable_editor),
            )
        } else {
            container(
                text("文件列表默认值暂未可视化，仍可在“高级 DSL”里直接编辑。")
                    .size(12)
                    .style(settings_muted_text_style),
            )
            .padding([10, 12])
            .style(value_card_style)
            .into()
        };

    column![
        build_editor_field(
            "支持文件类型",
            container(file_type_buttons.wrap()).width(Length::Fill).into(),
        ),
        custom_extensions_field,
        build_editor_field(
            "上传方式",
            row![
                build_start_variable_upload_method_button(
                    "local_file",
                    "本地上传",
                    upload_mode == "local_file",
                ),
                build_start_variable_upload_method_button(
                    "remote_url",
                    "URL",
                    upload_mode == "remote_url",
                ),
                build_start_variable_upload_method_button(
                    "all",
                    "两者都支持",
                    upload_mode == "all"
                ),
            ]
            .spacing(8)
            .into(),
        ),
        max_upload_count_field,
        default_value_field,
    ]
    .spacing(12)
    .into()
}

/// 构建 start variable card 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_start_variable_card<'a>(
    start_variable_editor: &'a super::state::WorkflowStartVariableEditorDraft,
) -> Element<'a, Message> {
    let variable = &start_variable_editor.variable;
    let advanced_hint = start_variable_advanced_hint(&variable.input_type);
    let default_value_error = start_variable_default_error(variable);
    let advanced_hint_card: Element<'a, Message> = if let Some(hint) = advanced_hint {
        text(hint).size(12).style(settings_muted_text_style).into()
    } else {
        container(Space::new().width(1).height(1)).into()
    };
    let type_specific_fields: Element<'a, Message> = if variable.input_type == "select" {
        column![
            build_start_variable_option_editor(variable),
            build_start_variable_select_default_field(variable),
        ]
        .spacing(12)
        .into()
    } else if matches!(variable.input_type.as_str(), "file" | "file-list") {
        build_start_variable_file_settings(start_variable_editor)
    } else if variable.input_type == "checkbox" {
        let options = vec!["默认勾选".to_string(), "不默认选中".to_string()];
        let selected = match variable.default_value.trim().to_ascii_lowercase().as_str() {
            "true" => Some("默认勾选".to_string()),
            "false" | "" => Some("不默认选中".to_string()),
            _ => None,
        };

        build_editor_field(
            "默认值",
            pick_list(options, selected, |value| {
                let normalized = if value == "默认勾选" { "true" } else { "false" };
                Message::WorkflowTool(WorkflowMessage::NodeEditorStartVariableEditorDefaultChanged(
                    normalized.to_string(),
                ))
            })
            .padding([10, 12])
            .text_size(13)
            .style(settings_pick_list_style)
            .menu_style(settings_pick_list_menu_style)
            .width(Length::Fill)
            .into(),
        )
    } else {
        let max_length_field: Element<'a, Message> =
            if matches!(variable.input_type.as_str(), "text-input" | "paragraph") {
                build_editor_field(
                    "最大长度",
                    workflow_text_input("请输入", &variable.max_length_input, |value| {
                        Message::WorkflowTool(
                            WorkflowMessage::NodeEditorStartVariableEditorMaxLengthChanged(value),
                        )
                    }),
                )
            } else {
                container(Space::new().width(1).height(1)).into()
            };

        let default_value_field: Element<'a, Message> = if variable.input_type == "paragraph" {
            build_editor_field_validated(
                "默认值",
                build_embedded_text_editor(
                    &start_variable_editor.default_value_editor,
                    "请输入",
                    WorkflowMessage::NodeEditorStartVariableEditorDefaultAction,
                    112.0,
                ),
                default_value_error,
            )
        } else {
            build_editor_field_validated(
                "默认值",
                workflow_text_input("请输入", &variable.default_value, |value| {
                    Message::WorkflowTool(
                        WorkflowMessage::NodeEditorStartVariableEditorDefaultChanged(value),
                    )
                }),
                default_value_error,
            )
        };

        column![max_length_field, default_value_field,].spacing(12).into()
    };

    container(
        column![
            build_editor_field(
                "字段类型",
                build_start_variable_type_selector(&variable.input_type)
            ),
            build_editor_field(
                "变量名称",
                workflow_text_input("请输入", &variable.variable, |value| {
                    Message::WorkflowTool(
                        WorkflowMessage::NodeEditorStartVariableEditorNameChanged(value),
                    )
                }),
            ),
            build_editor_field(
                "显示名称",
                workflow_text_input("请输入", &variable.label, |value| {
                    Message::WorkflowTool(
                        WorkflowMessage::NodeEditorStartVariableEditorLabelChanged(value),
                    )
                }),
            ),
            type_specific_fields,
            row![
                row![
                    checkbox(variable.required).label("").on_toggle(|value| {
                        Message::WorkflowTool(
                            WorkflowMessage::NodeEditorStartVariableEditorRequiredChanged(value),
                        )
                    }),
                    text("必填").size(13),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
                row![
                    checkbox(variable.hidden).label("").on_toggle(|value| {
                        Message::WorkflowTool(
                            WorkflowMessage::NodeEditorStartVariableEditorHiddenChanged(value),
                        )
                    }),
                    text("隐藏").size(13),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            ]
            .spacing(20)
            .align_y(Alignment::Center),
            advanced_hint_card,
        ]
        .spacing(12),
    )
    .into()
}

#[cfg(test)]
#[path = "start_inputs_tests.rs"]
mod start_inputs_tests;
