//! # Workflow 节点注册表
//!
//! 该模块维护节点类型注册信息，以及节点名称、图标、配色和系统变量映射。

use super::*;

pub fn workflow_system_variables(app_meta: &WorkflowAppMeta) -> Vec<WorkflowSystemVariable> {
    let mut variables = Vec::new();
    if is_chat_mode(&app_meta.mode) {
        variables.extend(CHAT_SYSTEM_VARIABLES);
    }
    variables.extend(BASE_SYSTEM_VARIABLES);
    variables
}

pub fn pretty_block_type(kind: &str) -> String {
    match kind.trim() {
        "start" => "开始".to_string(),
        "end" => "结束".to_string(),
        "answer" => "回复".to_string(),
        "llm" => "LLM".to_string(),
        "if-else" => "条件分支".to_string(),
        "code" => "代码".to_string(),
        "tool" => "工具".to_string(),
        "knowledge-retrieval" => "知识检索".to_string(),
        "question-classifier" => "问题分类".to_string(),
        "http-request" => "HTTP 请求".to_string(),
        "template-transform" => "模板转换".to_string(),
        "parameter-extractor" => "参数提取".to_string(),
        "iteration" => "迭代".to_string(),
        "loop" => "循环".to_string(),
        "variable-assigner" => "变量赋值".to_string(),
        "variable-aggregator" => "变量聚合".to_string(),
        "list-operator" => "列表操作".to_string(),
        "agent" => "Agent".to_string(),
        "document-extractor" => "文档提取".to_string(),
        other if other.is_empty() => "节点".to_string(),
        other => other.replace('-', " "),
    }
}

pub fn supported_node_types() -> &'static [WorkflowNodeTypeDescriptor] {
    &WORKFLOW_NODE_TYPES
}

pub fn workflow_node_icon(kind: &str) -> WorkflowNodeIconDescriptor {
    match kind.trim() {
        "start" => WorkflowNodeIconDescriptor { family: "lucide", name: "play" },
        "end" => WorkflowNodeIconDescriptor { family: "lucide", name: "circle-check" },
        "answer" => WorkflowNodeIconDescriptor { family: "lucide", name: "message-square-text" },
        "llm" => WorkflowNodeIconDescriptor { family: "lucide", name: "bot" },
        "if-else" => WorkflowNodeIconDescriptor { family: "lucide", name: "git-branch" },
        "code" => WorkflowNodeIconDescriptor { family: "lucide", name: "braces" },
        "tool" => WorkflowNodeIconDescriptor { family: "lucide", name: "wrench" },
        "knowledge-retrieval" => WorkflowNodeIconDescriptor { family: "lucide", name: "database" },
        "question-classifier" => {
            WorkflowNodeIconDescriptor { family: "lucide", name: "circle-question-mark" }
        }
        "http-request" => WorkflowNodeIconDescriptor { family: "lucide", name: "globe" },
        "template-transform" => {
            WorkflowNodeIconDescriptor { family: "lucide", name: "replace-all" }
        }
        "parameter-extractor" => WorkflowNodeIconDescriptor { family: "lucide", name: "scan-text" },
        "document-extractor" => WorkflowNodeIconDescriptor { family: "lucide", name: "file-text" },
        "iteration" => WorkflowNodeIconDescriptor { family: "lucide", name: "iteration-cw" },
        "loop" => WorkflowNodeIconDescriptor { family: "lucide", name: "refresh-cw" },
        "variable-assigner" => WorkflowNodeIconDescriptor { family: "lucide", name: "variable" },
        "variable-aggregator" => WorkflowNodeIconDescriptor { family: "lucide", name: "combine" },
        "list-operator" => WorkflowNodeIconDescriptor { family: "lucide", name: "list-filter" },
        "agent" => WorkflowNodeIconDescriptor { family: "lucide", name: "bot-message-square" },
        _ => WorkflowNodeIconDescriptor { family: "lucide", name: "workflow" },
    }
}

pub fn workflow_node_accent_color(kind: &str) -> Color {
    match kind.trim() {
        "start" => Color::from_rgb8(0x3B, 0x82, 0xF6),
        "end" => Color::from_rgb8(0x14, 0xB8, 0xA6),
        "answer" => Color::from_rgb8(0xF5, 0x9E, 0x0B),
        "llm" => Color::from_rgb8(0x8B, 0x5C, 0xF6),
        "if-else" => Color::from_rgb8(0x0E, 0xAE, 0xC7),
        "code" => Color::from_rgb8(0xF9, 0x73, 0x16),
        "tool" => Color::from_rgb8(0x63, 0x66, 0xF1),
        "knowledge-retrieval" => Color::from_rgb8(0xF5, 0x73, 0x2E),
        "question-classifier" => Color::from_rgb8(0x06, 0xB6, 0xD4),
        "http-request" => Color::from_rgb8(0xEC, 0x48, 0x99),
        "template-transform" => Color::from_rgb8(0x8B, 0x5C, 0xF6),
        "parameter-extractor" => Color::from_rgb8(0x4F, 0x46, 0xE5),
        "document-extractor" => Color::from_rgb8(0x47, 0x72, 0x9D),
        "iteration" => Color::from_rgb8(0x10, 0xB9, 0x81),
        "loop" => Color::from_rgb8(0x38, 0xB2, 0xAC),
        "variable-assigner" => Color::from_rgb8(0x22, 0xC5, 0x5E),
        "variable-aggregator" => Color::from_rgb8(0x0F, 0x76, 0x94),
        "list-operator" => Color::from_rgb8(0x25, 0x63, 0xEB),
        "agent" => Color::from_rgb8(0xE1, 0x1D, 0x48),
        _ => Color::from_rgb8(0x64, 0x74, 0x8B),
    }
}
