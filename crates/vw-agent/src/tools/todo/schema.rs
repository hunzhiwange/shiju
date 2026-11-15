use serde::Deserialize;
use vw_shared::todo::{de_opt_string_or_number, default_todo_priority, default_todo_status};

/// 写入操作的参数结构
///
/// 包含待办事项列表和可选的合并模式标志。
/// 当 merge 为 true 时，增量更新现有待办事项；为 false 时，完全替换。
#[derive(Debug, Clone, Deserialize)]
pub(super) struct WriteArgs {
    /// 待办事项数据数组（原始 JSON 值）
    pub(super) todos: Vec<serde_json::Value>,
    /// 是否使用合并模式更新待办事项，默认为 false（替换模式）
    #[serde(default)]
    pub(super) merge: bool,
}

/// 待办事项补丁结构
///
/// 用于合并模式下对待办事项进行部分更新。
/// 所有字段均为可选，仅更新提供的字段。
#[derive(Debug, Clone, Deserialize)]
pub(super) struct TodoPatch {
    /// 待办事项 ID（可选，用于定位要更新的项）
    #[serde(default, deserialize_with = "de_opt_string_or_number")]
    pub(super) id: Option<String>,
    /// 新的内容描述（可选）
    #[serde(default)]
    pub(super) content: Option<String>,
    /// 新的状态（可选）
    #[serde(default)]
    pub(super) status: Option<String>,
    /// 新的优先级（可选）
    #[serde(default)]
    pub(super) priority: Option<String>,
}

/// 待办事项输入结构
///
/// 用于解析用户传入的待办项数据，所有字段都有默认值处理。
#[derive(Debug, Clone, Deserialize)]
pub(super) struct TodoInput {
    /// 待办事项内容（必填）
    pub(super) content: String,
    /// 状态，默认为 "pending"
    #[serde(default = "default_todo_status")]
    pub(super) status: String,
    /// 优先级，默认为 "medium"
    #[serde(default = "default_todo_priority")]
    pub(super) priority: String,
    /// 可选的 ID，若不提供则自动分配
    #[serde(default, deserialize_with = "de_opt_string_or_number")]
    pub(super) id: Option<String>,
}
