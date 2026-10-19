//! # Workflow 默认配置
//!
//! 该模块定义不同节点类型的默认尺寸、默认句柄方向和默认数据结构。

use super::*;

fn default_start_variables_value() -> Value {
    Value::Sequence(vec![
        yaml_map(vec![
            ("label", Value::String("Query".to_string())),
            ("required", Value::Bool(true)),
            ("type", Value::String("paragraph".to_string())),
            ("variable", Value::String("query".to_string())),
        ]),
        yaml_map(vec![
            (
                "allowed_file_types",
                Value::Sequence(vec![
                    Value::String("document".to_string()),
                    Value::String("image".to_string()),
                    Value::String("audio".to_string()),
                    Value::String("video".to_string()),
                ]),
            ),
            (
                "allowed_file_upload_methods",
                Value::Sequence(vec![
                    Value::String("local_file".to_string()),
                    Value::String("remote_url".to_string()),
                ]),
            ),
            ("label", Value::String("Files".to_string())),
            ("type", Value::String("file-list".to_string())),
            ("variable", Value::String("files".to_string())),
        ]),
    ])
}

pub(super) fn blank_node_value(
    block_type: &str,
    node_id: String,
    position: Point,
    z_index: f32,
) -> Value {
    let size = default_node_size(block_type);
    yaml_map(vec![
        ("data", default_node_data_value(block_type)),
        ("height", yaml_value(size.height as f64)),
        ("id", Value::String(node_id)),
        ("position", point_value(position.x, position.y)),
        ("positionAbsolute", point_value(position.x, position.y)),
        ("selected", Value::Bool(false)),
        ("sourcePosition", Value::String(default_source_side(block_type).to_string())),
        ("targetPosition", Value::String(default_target_side(block_type).to_string())),
        ("type", Value::String("custom".to_string())),
        ("width", yaml_value(size.width as f64)),
        ("zIndex", yaml_value(z_index as f64)),
    ])
}

pub(super) fn default_node_size(block_type: &str) -> Size {
    match block_type {
        "start" => Size::new(240.0, 100.0),
        "answer" => Size::new(240.0, 96.0),
        "if-else" => Size::new(242.0, 180.0),
        "iteration" | "loop" => Size::new(320.0, 220.0),
        _ => Size::new(242.0, 82.0),
    }
}

pub(super) fn default_source_side(block_type: &str) -> &'static str {
    match block_type {
        "iteration" | "loop" => "right",
        _ => "right",
    }
}

pub(super) fn default_target_side(block_type: &str) -> &'static str {
    match block_type {
        "start" => "left",
        _ => "left",
    }
}

pub(super) fn default_node_data_value(block_type: &str) -> Value {
    let title = pretty_block_type(block_type);

    match block_type {
        "start" => yaml_map(vec![
            ("desc", Value::String(String::new())),
            ("selected", Value::Bool(false)),
            ("title", Value::String(title)),
            ("type", Value::String("start".to_string())),
            ("variables", default_start_variables_value()),
        ]),
        "answer" => yaml_map(vec![
            ("answer", Value::String("你好，这是一条新的回复节点。".to_string())),
            ("desc", Value::String(String::new())),
            ("selected", Value::Bool(false)),
            ("title", Value::String(title)),
            ("type", Value::String("answer".to_string())),
            ("variables", Value::Sequence(Vec::new())),
        ]),
        "end" => yaml_map(vec![
            ("desc", Value::String(String::new())),
            ("outputs", Value::Sequence(Vec::new())),
            ("selected", Value::Bool(false)),
            ("title", Value::String(title)),
            ("type", Value::String("end".to_string())),
        ]),
        "template-transform" => yaml_map(vec![
            ("desc", Value::String(String::new())),
            ("selected", Value::Bool(false)),
            ("template", Value::String(String::new())),
            ("title", Value::String(title)),
            ("type", Value::String("template-transform".to_string())),
            ("variables", Value::Sequence(Vec::new())),
        ]),
        "document-extractor" => yaml_map(vec![
            ("desc", Value::String(String::new())),
            ("selected", Value::Bool(false)),
            ("title", Value::String(title)),
            ("type", Value::String("document-extractor".to_string())),
            ("variable_selector", Value::Sequence(Vec::new())),
        ]),
        "code" => yaml_map(vec![
            ("code", Value::String("def main():\n    return {}\n".to_string())),
            ("code_language", Value::String("python3".to_string())),
            ("desc", Value::String(String::new())),
            ("outputs", yaml_map(vec![])),
            ("selected", Value::Bool(false)),
            ("title", Value::String(title)),
            ("type", Value::String("code".to_string())),
            ("variables", Value::Sequence(Vec::new())),
        ]),
        "llm" => yaml_map(vec![
            (
                "context",
                yaml_map(vec![
                    ("enabled", Value::Bool(false)),
                    ("variable_selector", Value::Sequence(Vec::new())),
                ]),
            ),
            ("desc", Value::String(String::new())),
            (
                "prompt_template",
                Value::Sequence(vec![yaml_map(vec![
                    ("role", Value::String("system".to_string())),
                    ("text", Value::String(String::new())),
                ])]),
            ),
            ("selected", Value::Bool(false)),
            ("title", Value::String(title)),
            ("type", Value::String("llm".to_string())),
            ("variables", Value::Sequence(Vec::new())),
            ("vision", yaml_map(vec![("enabled", Value::Bool(false))])),
        ]),
        "if-else" => yaml_map(vec![
            (
                "cases",
                Value::Sequence(vec![
                    yaml_map(vec![
                        ("case_id", Value::String("true".to_string())),
                        ("conditions", Value::Sequence(Vec::new())),
                        ("id", Value::String("true".to_string())),
                        ("logical_operator", Value::String("and".to_string())),
                    ]),
                    yaml_map(vec![
                        ("case_id", Value::String("false".to_string())),
                        ("conditions", Value::Sequence(Vec::new())),
                        ("id", Value::String("false".to_string())),
                        ("logical_operator", Value::String("and".to_string())),
                    ]),
                ]),
            ),
            ("desc", Value::String(String::new())),
            ("selected", Value::Bool(false)),
            ("title", Value::String(title)),
            ("type", Value::String("if-else".to_string())),
        ]),
        "knowledge-retrieval" => yaml_map(vec![
            ("dataset_ids", Value::Sequence(Vec::new())),
            ("desc", Value::String(String::new())),
            (
                "multiple_retrieval_config",
                yaml_map(vec![
                    ("reranking_enable", Value::Bool(false)),
                    ("score_threshold", Value::Null),
                    ("top_k", yaml_value(5_u64)),
                ]),
            ),
            ("query_attachment_selector", Value::Sequence(Vec::new())),
            ("query_variable_selector", Value::Sequence(Vec::new())),
            ("retrieval_mode", Value::String("multiple".to_string())),
            ("selected", Value::Bool(false)),
            (
                "single_retrieval_config",
                yaml_map(vec![(
                    "model",
                    yaml_map(vec![
                        ("completion_params", yaml_map(vec![])),
                        ("mode", Value::String("chat".to_string())),
                        ("name", Value::String(String::new())),
                        ("provider", Value::String(String::new())),
                    ]),
                )]),
            ),
            ("title", Value::String(title)),
            ("type", Value::String("knowledge-retrieval".to_string())),
        ]),
        "tool" => yaml_map(vec![
            ("credential_id", Value::String(String::new())),
            ("desc", Value::String(String::new())),
            ("plugin_unique_identifier", Value::String(String::new())),
            ("provider_id", Value::String(String::new())),
            ("provider_name", Value::String(String::new())),
            ("provider_type", Value::String(String::new())),
            ("selected", Value::Bool(false)),
            ("title", Value::String(title)),
            ("tool_configurations", yaml_map(vec![])),
            ("tool_description", Value::String(String::new())),
            ("tool_label", Value::String(String::new())),
            ("tool_name", Value::String(String::new())),
            ("tool_node_version", Value::String("2".to_string())),
            ("tool_parameters", yaml_map(vec![])),
            ("type", Value::String("tool".to_string())),
        ]),
        "agent" => yaml_map(vec![
            ("agent_parameters", yaml_map(vec![])),
            ("agent_strategy_label", Value::String(String::new())),
            ("agent_strategy_name", Value::String(String::new())),
            ("agent_strategy_provider_name", Value::String(String::new())),
            ("desc", Value::String(String::new())),
            (
                "memory",
                yaml_map(vec![
                    ("query_prompt_template", Value::String(String::new())),
                    (
                        "window",
                        yaml_map(vec![
                            ("enabled", Value::Bool(false)),
                            ("size", yaml_value(3_u64)),
                        ]),
                    ),
                ]),
            ),
            ("output_schema", yaml_map(vec![])),
            ("plugin_unique_identifier", Value::String(String::new())),
            ("selected", Value::Bool(false)),
            ("title", Value::String(title)),
            ("tool_node_version", Value::String("2".to_string())),
            ("type", Value::String("agent".to_string())),
        ]),
        "iteration" | "loop" => yaml_map(vec![
            ("desc", Value::String(String::new())),
            ("selected", Value::Bool(false)),
            ("title", Value::String(title)),
            ("type", Value::String(block_type.to_string())),
        ]),
        _ => yaml_map(vec![
            ("desc", Value::String(String::new())),
            ("selected", Value::Bool(false)),
            ("title", Value::String(title)),
            ("type", Value::String(block_type.to_string())),
        ]),
    }
}
