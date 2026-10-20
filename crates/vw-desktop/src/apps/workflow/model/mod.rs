//! # Workflow 数据模型
//!
//! 该模块集中声明工作流数据模型、节点与连线类型、系统变量以及模型层导出接口。

use iced::{Color, Point, Rectangle, Size};
use serde::{Deserialize, Serialize};
use serde_yaml::{Mapping, Value};
use std::collections::HashMap;
use std::path::Path;

mod creation;
mod defaults;
mod dify;
mod loader;
mod registry;
mod save;

#[cfg(test)]
#[path = "creation_tests.rs"]
mod creation_tests;
#[cfg(test)]
#[path = "defaults_tests.rs"]
mod defaults_tests;
#[cfg(test)]
#[path = "dify_node_fixtures_tests.rs"]
mod dify_node_fixtures_tests;
#[cfg(test)]
#[path = "dify_tests.rs"]
mod dify_tests;
#[cfg(test)]
#[path = "loader_tests.rs"]
mod loader_tests;
#[cfg(test)]
#[path = "registry_tests.rs"]
mod registry_tests;
#[cfg(test)]
#[path = "save_tests.rs"]
mod save_tests;

use defaults::*;
use dify::*;
use loader::*;
use save::*;

pub use creation::{
    create_blank_workflow, create_node_from_type, default_node_data_yaml, load_document_from_path,
    load_document_from_text, node_data_yaml, rebuild_node_from_parts,
};
pub use loader::{load_document_from_value, serialize_workflow_yaml, suggested_workflow_file_name};
pub use registry::{
    pretty_block_type, supported_node_types, workflow_node_accent_color, workflow_node_icon,
    workflow_system_variables,
};
pub(crate) use save::yaml_map;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowHandleSide {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowHandleKind {
    Source,
    Target,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowHandle {
    pub id: String,
    pub label: String,
    pub kind: WorkflowHandleKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowConnectionEndpoint {
    pub node_id: String,
    pub handle_id: String,
    pub kind: WorkflowHandleKind,
}

#[derive(Debug, Clone)]
pub struct WorkflowConnectionDraft {
    pub from: WorkflowConnectionEndpoint,
    pub cursor_world: Point,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorkflowViewport {
    pub x: f32,
    pub y: f32,
    pub zoom: f32,
}

impl Default for WorkflowViewport {
    fn default() -> Self {
        Self { x: 120.0, y: 120.0, zoom: 1.0 }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowAppMeta {
    pub name: String,
    pub description: String,
    pub icon: String,
    pub icon_background: String,
    pub mode: String,
    pub use_icon_as_answer_icon: bool,
    pub max_active_requests: u32,
}

impl Default for WorkflowAppMeta {
    fn default() -> Self {
        Self {
            name: "未命名应用".to_string(),
            description: String::new(),
            icon: "🤖".to_string(),
            icon_background: "#FFEAD5".to_string(),
            mode: "advanced-chat".to_string(),
            use_icon_as_answer_icon: false,
            max_active_requests: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowEnvironmentVariable {
    pub id: String,
    pub name: String,
    pub value_type: String,
    pub value: Value,
    pub description: String,
    pub raw_variable: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowConversationVariable {
    pub id: String,
    pub name: String,
    pub value_type: String,
    pub value: Value,
    pub description: String,
    pub raw_variable: Value,
}

#[derive(Debug, Clone, Copy)]
pub struct WorkflowSystemVariable {
    pub name: &'static str,
    pub value_type: &'static str,
    pub description: &'static str,
}

const BASE_SYSTEM_VARIABLES: [WorkflowSystemVariable; 4] = [
    WorkflowSystemVariable {
        name: "sys.user_id",
        value_type: "string",
        description: "当前调用用户的唯一 ID。",
    },
    WorkflowSystemVariable {
        name: "sys.app_id",
        value_type: "string",
        description: "当前应用的唯一 ID。",
    },
    WorkflowSystemVariable {
        name: "sys.workflow_id",
        value_type: "string",
        description: "当前工作流草稿或版本的唯一 ID。",
    },
    WorkflowSystemVariable {
        name: "sys.workflow_run_id",
        value_type: "string",
        description: "当前这一次执行运行的唯一 ID。",
    },
];

const CHAT_SYSTEM_VARIABLES: [WorkflowSystemVariable; 2] = [
    WorkflowSystemVariable {
        name: "sys.dialogue_count",
        value_type: "number",
        description: "当前对话累计轮数。",
    },
    WorkflowSystemVariable {
        name: "sys.conversation_id",
        value_type: "string",
        description: "当前会话的唯一 ID。",
    },
];

const START_NODE_MIN_HEIGHT_EMPTY: f32 = 120.0;
const START_NODE_TOP_PADDING: f32 = 18.0;
const START_NODE_HEADER_HEIGHT: f32 = 24.0;
const START_NODE_VARIABLE_LIST_GAP: f32 = 14.0;
const START_NODE_VARIABLE_ROW_HEIGHT: f32 = 34.0;
const START_NODE_VARIABLE_ROW_GAP: f32 = 8.0;
const START_NODE_BOTTOM_PADDING: f32 = 16.0;

#[derive(Debug, Clone, Copy)]
pub struct WorkflowNodeIconDescriptor {
    pub family: &'static str,
    pub name: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct WorkflowNodeTypeDescriptor {
    pub block_type: &'static str,
    pub label: &'static str,
    pub summary: &'static str,
    pub icon: WorkflowNodeIconDescriptor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowStartVariableSummary {
    pub name: String,
    pub value_type: String,
}

const WORKFLOW_NODE_TYPES: [WorkflowNodeTypeDescriptor; 19] = [
    WorkflowNodeTypeDescriptor {
        block_type: "start",
        label: "开始",
        summary: "定义工作流输入变量与起始入口。",
        icon: WorkflowNodeIconDescriptor { family: "lucide", name: "play" },
    },
    WorkflowNodeTypeDescriptor {
        block_type: "llm",
        label: "LLM",
        summary: "提示词、模型与上下文配置。",
        icon: WorkflowNodeIconDescriptor { family: "lucide", name: "bot" },
    },
    WorkflowNodeTypeDescriptor {
        block_type: "answer",
        label: "回复",
        summary: "直接输出文本或模板结果。",
        icon: WorkflowNodeIconDescriptor { family: "lucide", name: "message-square-text" },
    },
    WorkflowNodeTypeDescriptor {
        block_type: "end",
        label: "结束",
        summary: "结束工作流并返回结构化输出。",
        icon: WorkflowNodeIconDescriptor { family: "lucide", name: "circle-check" },
    },
    WorkflowNodeTypeDescriptor {
        block_type: "if-else",
        label: "条件分支",
        summary: "通过 cases 定义多个条件分支。",
        icon: WorkflowNodeIconDescriptor { family: "lucide", name: "git-branch" },
    },
    WorkflowNodeTypeDescriptor {
        block_type: "code",
        label: "代码",
        summary: "编写代码处理输入与输出变量。",
        icon: WorkflowNodeIconDescriptor { family: "lucide", name: "braces" },
    },
    WorkflowNodeTypeDescriptor {
        block_type: "tool",
        label: "工具",
        summary: "调用工具或外部能力。",
        icon: WorkflowNodeIconDescriptor { family: "lucide", name: "wrench" },
    },
    WorkflowNodeTypeDescriptor {
        block_type: "knowledge-retrieval",
        label: "知识检索",
        summary: "检索知识库并产出召回结果。",
        icon: WorkflowNodeIconDescriptor { family: "lucide", name: "database" },
    },
    WorkflowNodeTypeDescriptor {
        block_type: "question-classifier",
        label: "问题分类",
        summary: "基于问题内容进行分类路由。",
        icon: WorkflowNodeIconDescriptor { family: "lucide", name: "circle-question-mark" },
    },
    WorkflowNodeTypeDescriptor {
        block_type: "parameter-extractor",
        label: "参数提取",
        summary: "从文本中提取结构化参数。",
        icon: WorkflowNodeIconDescriptor { family: "lucide", name: "scan-text" },
    },
    WorkflowNodeTypeDescriptor {
        block_type: "http-request",
        label: "HTTP 请求",
        summary: "向外部接口发送请求。",
        icon: WorkflowNodeIconDescriptor { family: "lucide", name: "globe" },
    },
    WorkflowNodeTypeDescriptor {
        block_type: "template-transform",
        label: "模板转换",
        summary: "使用模板组合变量并生成文本。",
        icon: WorkflowNodeIconDescriptor { family: "lucide", name: "replace-all" },
    },
    WorkflowNodeTypeDescriptor {
        block_type: "document-extractor",
        label: "文档提取",
        summary: "从文件变量中提取文档文本。",
        icon: WorkflowNodeIconDescriptor { family: "lucide", name: "file-text" },
    },
    WorkflowNodeTypeDescriptor {
        block_type: "iteration",
        label: "迭代",
        summary: "容器型节点，适合批量处理。",
        icon: WorkflowNodeIconDescriptor { family: "lucide", name: "iteration-cw" },
    },
    WorkflowNodeTypeDescriptor {
        block_type: "loop",
        label: "循环",
        summary: "容器型节点，适合重复执行。",
        icon: WorkflowNodeIconDescriptor { family: "lucide", name: "refresh-cw" },
    },
    WorkflowNodeTypeDescriptor {
        block_type: "variable-assigner",
        label: "变量赋值",
        summary: "写入或覆盖变量值。",
        icon: WorkflowNodeIconDescriptor { family: "lucide", name: "variable" },
    },
    WorkflowNodeTypeDescriptor {
        block_type: "variable-aggregator",
        label: "变量聚合",
        summary: "把多个变量汇总为一个输出。",
        icon: WorkflowNodeIconDescriptor { family: "lucide", name: "combine" },
    },
    WorkflowNodeTypeDescriptor {
        block_type: "list-operator",
        label: "列表操作",
        summary: "过滤、排序并截取列表变量。",
        icon: WorkflowNodeIconDescriptor { family: "lucide", name: "list-filter" },
    },
    WorkflowNodeTypeDescriptor {
        block_type: "agent",
        label: "Agent",
        summary: "Agent 策略与工具编排节点。",
        icon: WorkflowNodeIconDescriptor { family: "lucide", name: "bot-message-square" },
    },
];

#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowNode {
    pub id: String,
    pub block_type: String,
    pub title: String,
    pub description: String,
    pub position: Point,
    pub size: Size,
    pub parent_id: Option<String>,
    pub selected: bool,
    pub source_side: WorkflowHandleSide,
    pub target_side: WorkflowHandleSide,
    pub source_handles: Vec<WorkflowHandle>,
    pub target_handles: Vec<WorkflowHandle>,
    pub z_index: f32,
    pub raw_node: Value,
}

impl WorkflowNode {
    pub fn rect_world(&self) -> Rectangle {
        Rectangle::new(self.position, self.size)
    }

    pub fn is_group(&self) -> bool {
        matches!(self.block_type.as_str(), "iteration" | "loop") || self.size.height >= 140.0
    }

    pub fn handle(&self, kind: WorkflowHandleKind, handle_id: &str) -> Option<&WorkflowHandle> {
        let handles = match kind {
            WorkflowHandleKind::Source => &self.source_handles,
            WorkflowHandleKind::Target => &self.target_handles,
        };

        handles.iter().find(|handle| handle.id == handle_id)
    }
}

pub fn workflow_start_node_variables(node: &WorkflowNode) -> Vec<WorkflowStartVariableSummary> {
    if node.block_type != "start" {
        return Vec::new();
    }

    workflow_start_node_variables_from_raw(&node.raw_node)
}

pub fn workflow_start_node_min_height(raw_node: &Value) -> f32 {
    let variable_count = workflow_start_node_variables_from_raw(raw_node).len();
    if variable_count == 0 {
        return START_NODE_MIN_HEIGHT_EMPTY;
    }

    (START_NODE_TOP_PADDING
        + START_NODE_HEADER_HEIGHT
        + START_NODE_VARIABLE_LIST_GAP
        + variable_count as f32 * START_NODE_VARIABLE_ROW_HEIGHT
        + variable_count.saturating_sub(1) as f32 * START_NODE_VARIABLE_ROW_GAP
        + START_NODE_BOTTOM_PADDING)
        .max(START_NODE_MIN_HEIGHT_EMPTY)
}

fn workflow_start_node_variables_from_raw(raw_node: &Value) -> Vec<WorkflowStartVariableSummary> {
    let Some(variable_values) = raw_node
        .as_mapping()
        .and_then(|node| node.get(&Value::String("data".to_string())))
        .and_then(Value::as_mapping)
        .and_then(|data| data.get(&Value::String("variables".to_string())))
        .and_then(Value::as_sequence)
    else {
        return Vec::new();
    };

    variable_values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let variable_map = value.as_mapping();
            let variable_name = variable_map
                .and_then(|map| map.get(&Value::String("variable".to_string())))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty());
            let variable_label = variable_map
                .and_then(|map| map.get(&Value::String("label".to_string())))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty());
            let input_type = variable_map
                .and_then(|map| map.get(&Value::String("type".to_string())))
                .and_then(Value::as_str)
                .unwrap_or("text-input");
            let display_name = variable_name
                .or(variable_label)
                .map(str::to_string)
                .unwrap_or_else(|| format!("变量 {}", index + 1));

            WorkflowStartVariableSummary {
                name: display_name,
                value_type: workflow_start_variable_value_type(input_type).to_string(),
            }
        })
        .collect()
}

fn workflow_start_variable_value_type(input_type: &str) -> &'static str {
    match input_type {
        "number" => "number",
        "checkbox" => "boolean",
        "file" => "file",
        "file-list" => "array[file]",
        _ => "string",
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub source_handle: Option<String>,
    pub target_handle: Option<String>,
    pub source_type: String,
    pub target_type: String,
    pub selected: bool,
    pub z_index: f32,
    pub raw_edge: Value,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct WorkflowDocument {
    pub name: String,
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
    pub viewport: WorkflowViewport,
}

impl WorkflowDocument {
    pub fn bounds(&self) -> Option<Rectangle> {
        let mut nodes = self.nodes.iter();
        let first = nodes.next()?;
        let first_rect = first.rect_world();

        let mut min_x = first_rect.x;
        let mut min_y = first_rect.y;
        let mut max_x = first_rect.x + first_rect.width;
        let mut max_y = first_rect.y + first_rect.height;

        for node in nodes {
            let rect = node.rect_world();
            min_x = min_x.min(rect.x);
            min_y = min_y.min(rect.y);
            max_x = max_x.max(rect.x + rect.width);
            max_y = max_y.max(rect.y + rect.height);
        }

        Some(Rectangle {
            x: min_x,
            y: min_y,
            width: (max_x - min_x).max(1.0),
            height: (max_y - min_y).max(1.0),
        })
    }

    pub fn node(&self, id: &str) -> Option<&WorkflowNode> {
        self.nodes.iter().find(|node| node.id == id)
    }

    pub fn node_mut(&mut self, id: &str) -> Option<&mut WorkflowNode> {
        self.nodes.iter_mut().find(|node| node.id == id)
    }

    pub fn edge(&self, id: &str) -> Option<&WorkflowEdge> {
        self.edges.iter().find(|edge| edge.id == id)
    }

    pub fn remove_edge(&mut self, id: &str) -> Option<WorkflowEdge> {
        let index = self.edges.iter().position(|edge| edge.id == id)?;
        Some(self.edges.remove(index))
    }

    pub fn group_child_count(&self, parent_id: &str) -> usize {
        self.nodes.iter().filter(|node| node.parent_id.as_deref() == Some(parent_id)).count()
    }

    pub fn ancestor_ids(&self, node_id: &str) -> Vec<String> {
        let mut ancestors = Vec::new();
        let mut current = self.node(node_id).and_then(|node| node.parent_id.clone());

        while let Some(parent_id) = current {
            ancestors.push(parent_id.clone());
            current = self.node(&parent_id).and_then(|node| node.parent_id.clone());
        }

        ancestors
    }

    pub fn descendant_ids(&self, parent_id: &str) -> Vec<String> {
        let mut descendants = Vec::new();
        let mut frontier = vec![parent_id.to_string()];

        while let Some(current_id) = frontier.pop() {
            for child in self
                .nodes
                .iter()
                .filter(|node| node.parent_id.as_deref() == Some(current_id.as_str()))
            {
                descendants.push(child.id.clone());
                frontier.push(child.id.clone());
            }
        }

        descendants
    }
}

#[derive(Debug, Clone)]
pub struct LoadedWorkflow {
    pub local_uuid: Option<String>,
    pub source_path: Option<String>,
    pub source_name: String,
    pub app_meta: WorkflowAppMeta,
    pub document: WorkflowDocument,
    pub environment_variables: Vec<WorkflowEnvironmentVariable>,
    pub conversation_variables: Vec<WorkflowConversationVariable>,
    pub had_viewport: bool,
    pub raw_root: Value,
}
