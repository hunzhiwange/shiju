//! Code node configuration panel.
use super::*;

/// 构建 code visual section 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub fn build_code_visual_section<'a>(
    i18n: desktop_i18n::I18n,
    state: &'a WorkflowState,
    editor: &'a super::state::WorkflowNodeEditorDraft,
    validation: &'a super::state::WorkflowNodeEditorValidation,
    language: &'a str,
    inputs: &'a [WorkflowCodeVariableDraft],
    code_editor: &'a text_editor::Content,
    outputs: &'a [WorkflowCodeOutputDraft],
    retry_config: WorkflowNodeRetryDraft,
    error_strategy: &'a str,
    default_value_editor: &'a text_editor::Content,
) -> Element<'a, Message> {
    let language_options = code_language_options().to_vec();
    let selected_language = code_picker_option(language, &language_options);
    let variable_options = build_code_variable_reference_options(state, editor);
    let output_type_options = code_output_type_options().to_vec();
    let error_strategy_options = code_error_strategy_options(i18n).to_vec();
    let selected_error_strategy = code_picker_option(error_strategy, &error_strategy_options);

    let input_list: Element<'a, Message> = if inputs.is_empty() {
        text(i18n.__("当前没有输入变量，可添加已有流程变量或系统变量作为代码参数。"))
            .size(12)
            .style(settings_muted_text_style)
            .into()
    } else {
        column(
            inputs
                .iter()
                .enumerate()
                .map(|(index, input)| {
                    build_code_input_variable_row(i18n, index, input, &variable_options, validation)
                })
                .collect::<Vec<Element<'a, Message>>>(),
        )
        .spacing(8)
        .into()
    };

    let output_list: Element<'a, Message> = if outputs.is_empty() {
        text(i18n.__("至少新增一个输出变量，代码节点的返回值会按这里声明的类型导出。"))
            .size(12)
            .style(settings_muted_text_style)
            .into()
    } else {
        column(
            outputs
                .iter()
                .enumerate()
                .map(|(index, output)| {
                    build_code_output_variable_row(
                        i18n,
                        index,
                        output,
                        &output_type_options,
                        validation,
                    )
                })
                .collect::<Vec<Element<'a, Message>>>(),
        )
        .spacing(8)
        .into()
    };

    let retry_error = validation
        .localized_error_for(i18n, "code.retry.max_retries")
        .or_else(|| validation.localized_error_for(i18n, "code.retry.retry_interval"));
    let error_strategy_hint = match error_strategy {
        "default-value" => i18n.__("节点发生异常时，使用下面的默认输出继续流转。"),
        "fail-branch" => {
            i18n.__("节点发生异常时，会暴露失败分支，可在“下一步”里继续编排处理节点。")
        }
        _ => i18n.__("发生异常且未处理时，节点将停止运行。"),
    };

    column![
        build_editor_field(
            i18n.__("输入变量"),
            column![
                input_list,
                button(text(i18n.__("新增输入变量")).size(12))
                    .style(rounded_action_btn_style)
                    .padding([8, 12])
                    .on_press(
                        WorkflowMessage::NodeEditorCodeAddInputVariable),
            ]
            .spacing(8)
            .into(),
        ),
        build_editor_field_validated(
            i18n.__("代码语言"),
            settings_select(language_options, selected_language, |option| {
                WorkflowMessage::NodeEditorCodeLanguageChanged(
                    option.value.to_string(),
                )
            })
            .width(Length::Fill)
            .into(),
            validation.localized_error_for(i18n, "code.language"),
        ),
        build_editor_field_validated(
            i18n.__("代码内容"),
            build_embedded_text_editor(
                code_editor,
                i18n.__("输入代码体"),
                WorkflowMessage::NodeEditorCodeAction,
                220.0,
            ),
            validation.localized_error_for(i18n, "code.body"),
        ),
        build_editor_field_validated(
            i18n.__("输出变量"),
            column![
                output_list,
                button(text(i18n.__("新增输出变量")).size(12))
                    .style(rounded_action_btn_style)
                    .padding([8, 12])
                    .on_press(
                        WorkflowMessage::NodeEditorCodeAddOutputVariable),
            ]
            .spacing(8)
            .into(),
            validation.localized_error_for(i18n, "code.outputs"),
        ),
        build_editor_field_validated(
            i18n.__("失败时重试"),
            column![
                row![
                    text(i18n.__("启用失败重试")).size(12),
                    Space::new().width(Length::Fill),
                    settings_toggle(retry_config.enabled).on_toggle(|value|
                        WorkflowMessage::NodeEditorCodeRetryEnabledChanged(value)),
                ]
                .align_y(Alignment::Center),
                {
                    let retry_detail: Element<'a, Message> = if retry_config.enabled {
                        column![
                            row![
                                text(i18n.__("最大重试次数"))
                                    .size(12)
                                    .style(settings_muted_text_style)
                                    .width(Length::Fixed(88.0)),
                                slider(
                                    1.0..=10.0,
                                    retry_config.max_retries as f32,
                                    |value: f32| {

                                            WorkflowMessage::NodeEditorCodeRetryMaxRetriesChanged(
                                                (value.round() as u8).clamp(1, 10),
                                            )
                                    }
                                )
                                .step(1.0_f32)
                                .width(Length::Fill),
                                container(text(retry_config.max_retries.to_string()).size(15))
                                    .padding([6, 10])
                                    .width(Length::Fixed(56.0))
                                    .style(value_card_style),
                                text(i18n.__("次")).size(12).style(settings_muted_text_style),
                            ]
                            .spacing(8)
                            .align_y(Alignment::Center),
                            row![
                                text(i18n.__("重试间隔"))
                                    .size(12)
                                    .style(settings_muted_text_style)
                                    .width(Length::Fixed(88.0)),
                                slider(
                                    100.0..=5000.0,
                                    retry_config.retry_interval as f32,
                                    |value: f32| {

                                            WorkflowMessage::NodeEditorCodeRetryIntervalChanged(
                                                ((value.round() as u16) / 100).max(1) * 100,
                                            )
                                    }
                                )
                                .step(100.0_f32)
                                .width(Length::Fill),
                                container(text(retry_config.retry_interval.to_string()).size(15))
                                    .padding([6, 10])
                                    .width(Length::Fixed(76.0))
                                    .style(value_card_style),
                                text(i18n.__("毫秒")).size(12).style(settings_muted_text_style),
                            ]
                            .spacing(8)
                            .align_y(Alignment::Center),
                        ]
                        .spacing(10)
                        .into()
                    } else {
                        container(
                            text(i18n.__("关闭后，代码节点发生错误会直接进入异常处理逻辑。"))
                                .size(12)
                                .style(settings_muted_text_style),
                        )
                        .padding([6, 0])
                        .into()
                    };
                    retry_detail
                },
            ]
            .spacing(10)
            .into(),
            retry_error,
        ),
        build_editor_field_validated(
            i18n.__("异常处理"),
            column![
                settings_select(error_strategy_options, selected_error_strategy, |option| {
                    WorkflowMessage::NodeEditorCodeErrorStrategyChanged(
                        option.value.to_string(),
                    )
                })
                .width(Length::Fill),
                text(error_strategy_hint).size(12).style(settings_muted_text_style),
            ]
            .spacing(8)
            .into(),
            validation.localized_error_for(i18n, "code.error_strategy"),
        ),
        if error_strategy == "default-value" {
            build_editor_field_validated(
                i18n.__("默认输出 YAML"),
                build_embedded_text_editor(
                    default_value_editor,
                    "- key: result\n  type: string\n  value: ''",
                    WorkflowMessage::NodeEditorCodeDefaultValueAction,
                    160.0,
                ),
                validation.localized_error_for(i18n, "code.default_value"),
            )
        } else {
            container(Space::new().width(1).height(1)).into()
        },
    ]
    .spacing(12)
    .into()
}
