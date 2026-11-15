use super::*;

pub fn start_variable_value_type_label(input_type: &str) -> &'static str {
    match input_type {
        "number" => "number",
        "checkbox" => "boolean",
        "file" => "file",
        "file-list" => "array[file]",
        _ => "string",
    }
}

pub fn start_builtin_variables(
    i18n: desktop_i18n::I18n,
    state: &WorkflowState,
) -> Vec<StartBuiltinVariableItem> {
    let is_chat_mode = state.active_meta().map(|meta| meta.mode.contains("chat")).unwrap_or(true);

    let mut items = Vec::new();
    if is_chat_mode {
        items.push(StartBuiltinVariableItem {
            name: "userinput.query",
            value_type: "String",
            description: i18n.__("用户当前这一轮输入的问题文本。"),
            legacy: false,
        });
    }
    items.push(StartBuiltinVariableItem {
        name: "userinput.files",
        value_type: "Array[File]",
        description: i18n.__("用户上传的文件列表，可作为文件型输入或后续节点的附件来源。"),
        legacy: !is_chat_mode,
    });

    items
}

pub fn start_variable_badge(label: &'static str) -> Element<'static, Message> {
    container(text(label).size(11)).padding([4, 8]).style(value_card_style).into()
}

pub fn start_variable_summary_error(
    i18n: desktop_i18n::I18n,
    index: usize,
    validation: &super::state::WorkflowNodeEditorValidation,
) -> Option<String> {
    let mut errors = Vec::new();

    for path in [
        format!("start.variables[{index}].label"),
        format!("start.variables[{index}].variable"),
        format!("start.variables[{index}].max_length"),
        format!("start.variables[{index}].options"),
        format!("start.variables[{index}].default"),
        format!("start.variables[{index}].allowed_file_types"),
        format!("start.variables[{index}].allowed_file_extensions"),
        format!("start.variables[{index}].allowed_file_upload_methods"),
    ] {
        if let Some(error) = validation.localized_error_for(i18n, &path) {
            errors.push(error);
        }
    }

    if errors.is_empty() { None } else { Some(errors.join(" · ")) }
}

pub fn start_variable_default_error(
    i18n: desktop_i18n::I18n,
    variable: &super::state::WorkflowStartVariableDraft,
) -> Option<&'static str> {
    if variable.input_type == "number"
        && !desktop_workflow_state::start_variable_validation::is_valid_start_variable_number_default_value(&variable.default_value)
    {
        Some(i18n.__("数字类型默认值必须是数字"))
    } else {
        None
    }
}

pub fn start_variable_advanced_hint(input_type: &str) -> Option<&'static str> {
    match input_type {
        _ => None,
    }
}

pub fn start_variable_type_options(
    i18n: desktop_i18n::I18n,
) -> Vec<StartVariableTypeOption> {
    vec![
        StartVariableTypeOption {
            input_type: "text-input",
            label: i18n.__("文本"),
            value_type: "string",
        },
        StartVariableTypeOption {
            input_type: "paragraph",
            label: i18n.__("段落"),
            value_type: "string",
        },
        StartVariableTypeOption {
            input_type: "select",
            label: i18n.__("下拉选项"),
            value_type: "string",
        },
        StartVariableTypeOption {
            input_type: "number",
            label: i18n.__("数字"),
            value_type: "number",
        },
        StartVariableTypeOption {
            input_type: "checkbox",
            label: i18n.__("复选框"),
            value_type: "boolean",
        },
        StartVariableTypeOption {
            input_type: "file",
            label: i18n.__("单文件"),
            value_type: "file",
        },
        StartVariableTypeOption {
            input_type: "file-list",
            label: i18n.__("文件列表"),
            value_type: "array[file]",
        },
    ]
}

pub fn selected_start_variable_type_option(
    i18n: desktop_i18n::I18n,
    input_type: &str,
) -> Option<StartVariableTypeOption> {
    start_variable_type_options(i18n).into_iter().find(|option| option.input_type == input_type)
}

pub fn build_start_variable_type_selector(
    i18n: desktop_i18n::I18n,
    input_type: &str,
) -> Element<'static, Message> {
    let options = start_variable_type_options(i18n);
    let selected = selected_start_variable_type_option(i18n, input_type);

    settings_select(options, selected, |option| {
        WorkflowMessage::NodeEditorStartVariableEditorTypeChanged(
            option.input_type.to_string(),
        )
    })
    .width(Length::Fill)
    .into()
}

pub fn start_variable_file_type_options(
    i18n: desktop_i18n::I18n,
) -> [(&'static str, &'static str, &'static str); 5] {
    [
        ("document", i18n.__("文档"), "PDF, DOCX"),
        ("image", i18n.__("图片"), "PNG, JPG"),
        ("audio", i18n.__("音频"), "MP3, WAV"),
        ("video", i18n.__("视频"), "MP4, MOV"),
        ("custom", i18n.__("自定义"), i18n.__("手动填写扩展名")),
    ]
}

#[cfg(test)]
#[path = "start_meta_tests.rs"]
mod start_meta_tests;
