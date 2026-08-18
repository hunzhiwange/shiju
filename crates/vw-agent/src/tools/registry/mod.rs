//! 工具注册表与规格生成模块
//!
//! 本模块负责根据模型和客户端类型动态过滤和生成工具规格列表。
//! 当前对外工具集默认采用 patch-first 策略，并在必要时继续保留
//! 基于提供商和客户端类型的可见性过滤。
//!
//! # 核心功能
//!
//! - 基于模型能力动态过滤工具列表
//! - 支持 VibeWindow 提供商的搜索工具启用判断
//! - 支持默认 patch-first 的外部工具集过滤
//! - 支持基于客户端类型的工具可见性控制
//! - 生成符合 OpenAI 函数调用格式的工具规格 JSON
//!
//! # 工具过滤策略
//!
//! 本模块实现了以下工具过滤策略：
//!
//! - **联网搜索工具**（websearch）：仅对 VibeWindow 提供商或启用 EXA 标志时可用
//! - **补丁工具**（apply_patch）：默认对外启用
//! - **提问工具**（question）：仅在 app、cli、desktop 客户端可用
//! - **LSP 工具**：仅在启用实验性 LSP 工具标志时可用
//!
//! # 使用示例
//!
//! ```ignore
//! use crate::app::agent::tools::registry::{specs, specs_json, openai_tools_json};
//!
//! // 获取特定模型的工具规格列表
//! let tool_specs = specs(Some("vibewindow/claude-3-opus"));
//!
//! // 获取工具规格的 JSON 表示
//! let json_value = specs_json(Some("openai/gpt-4"));
//!
//! // 获取 OpenAI 函数调用格式的工具 JSON
//! let openai_tools = openai_tools_json(Some("openai/gpt-4o"));
//! ```

use crate::app::agent::flag;
use crate::app::agent::provider::provider as model_provider;
use serde_json::json;
use vw_api_types::tools::{ListToolSpecsResponse, ToolSpecDto};

/// 判断是否应启用联网搜索工具（websearch）
///
/// # 参数
///
/// - `provider_id`: 模型提供商标识符
///
/// # 返回值
///
/// 如果提供商为 VibeWindow 或启用了 EXA 搜索标志，则返回 `true`；否则返回 `false`
///
/// # 示例
///
/// ```ignore
/// assert!(should_enable_search("vibewindow"));
/// assert!(should_enable_search("vibewindow-custom"));
/// ```
fn should_enable_search(provider_id: &str) -> bool {
    provider_id.starts_with("vibewindow") || *flag::VIBEWINDOW_ENABLE_EXA
}

/// 判断是否应对外暴露 apply_patch 工具
///
/// 当前对外工具集统一采用 patch-first 策略，默认优先暴露 apply_patch。
///
/// # 参数
///
/// - `model_id`: 模型标识符（不包含提供商前缀）
///
/// # 返回值
///
/// 当前始终返回 `true`，`model_id` 仅保留用于未来可能的灰度或例外控制。
///
/// # 示例
///
/// ```ignore
/// assert!(should_use_apply_patch("gpt-3.5-turbo"));
/// assert!(should_use_apply_patch("gpt-4o"));
/// assert!(should_use_apply_patch("claude-3-opus"));
/// ```
fn should_use_apply_patch(_model_id: &str) -> bool {
    true
}

fn filtered_specs(
    enable_search: bool,
    use_patch: bool,
    client: &str,
) -> Vec<crate::app::agent::tools::ToolSpec> {
    crate::app::agent::tools::tool_specs_for_context(
        &crate::app::agent::tools::ToolRuntimeContext::for_specs(),
    )
    .into_iter()
    .filter(|t| {
        if (t.id == "codesearch" || crate::app::agent::tools::is_web_search_tool_id(t.id.as_str()))
            && !enable_search
        {
            return false;
        }

        if t.id == "apply_patch" {
            return use_patch;
        }

        if crate::app::agent::tools::is_question_tool_id(t.id.as_str())
            && !should_enable_question_tool(client)
        {
            return false;
        }

        true
    })
    .collect()
}

/// 判断是否应启用提问工具（question）
///
/// 提问工具允许代理向用户提出澄清性问题，仅在交互式客户端中启用。
///
/// # 参数
///
/// - `client`: 客户端类型标识符
///
/// # 返回值
///
/// 如果客户端为 "app"、"cli" 或 "desktop"，则返回 `true`；否则返回 `false`
///
/// # 说明
///
/// 提问工具仅适用于具备用户交互界面的客户端环境。对于 API 或后台服务等
/// 非交互式环境，该工具将被禁用以避免阻塞执行流程。
///
/// # 示例
///
/// ```ignore
/// assert!(should_enable_question_tool("app"));
/// assert!(should_enable_question_tool("cli"));
/// assert!(should_enable_question_tool("desktop"));
/// assert!(!should_enable_question_tool("api"));
/// assert!(!should_enable_question_tool("webhook"));
/// ```
fn should_enable_question_tool(client: &str) -> bool {
    matches!(client, "app" | "cli" | "desktop")
}

/// 根据模型和客户端类型生成过滤后的工具规格列表
///
/// 此函数是工具注册表的核心函数，负责根据模型能力、客户端类型和特性标志
/// 动态生成可用的工具规格列表。
///
/// # 参数
///
/// - `model`: 可选的模型标识符字符串，格式为 "provider/model" 或 "model"
///   - 如果为 `None` 或空字符串，则返回默认的对外工具列表（patch-first）
///   - 如果包含 "/"，则解析为提供商 ID 和模型 ID
///   - 如果不包含 "/"，则模型 ID 为整个字符串，提供商 ID 为空
///
/// # 返回值
///
/// 返回过滤后的 `ToolSpec` 向量，包含当前上下文中可用的所有工具规格
///
/// # 过滤规则
///
/// 1. **联网搜索工具**（websearch）：仅在 VibeWindow 提供商或启用 EXA 时可用
/// 2. **补丁工具**（apply_patch）：默认对外启用
/// 3. **提问工具**（question）：仅在交互式客户端（app/cli/desktop）可用
/// 4. **LSP 工具**：仅在启用实验性 LSP 工具标志时可用
///
/// # 示例
///
/// ```ignore
/// use crate::app::agent::tools::registry::specs;
///
/// // 获取所有工具（无模型限制）
/// let all_tools = specs(None);
///
/// // 获取特定模型的可用工具
/// let claude_tools = specs(Some("vibewindow/claude-3-opus"));
///
/// // 获取 GPT-4 的可用工具
/// let gpt4_tools = specs(Some("openai/gpt-4"));
///
/// // 获取 GPT-3.5 的可用工具（支持 apply_patch）
/// let gpt35_tools = specs(Some("openai/gpt-3.5-turbo"));
/// ```
pub fn specs(model: Option<&str>) -> Vec<crate::app::agent::tools::ToolSpec> {
    let client = flag::vibewindow_client();

    // 如果模型为空或 None，返回默认的对外工具列表（patch-first）
    let Some(model) = model.filter(|s| !s.trim().is_empty()) else {
        return filtered_specs(true, true, &client);
    };

    // 去除模型字符串首尾空白
    let model = model.trim();

    // 解析模型标识符，分离提供商 ID 和模型 ID
    let (provider_id, model_id) = if model.contains('/') {
        // 格式为 "provider/model"，使用解析器提取
        let parsed = model_provider::parse_model(model);
        (parsed.provider_id, parsed.model_id)
    } else {
        // 格式为 "model"，提供商 ID 为空
        (String::new(), model.to_string())
    };

    // 根据提供商和模型 ID 决定各种工具的启用状态
    let enable_search = should_enable_search(&provider_id);
    let use_patch = should_use_apply_patch(&model_id);
    filtered_specs(enable_search, use_patch, &client)
}

/// 生成共享工具规格 DTO 列表。
pub fn spec_dtos(model: Option<&str>) -> Vec<ToolSpecDto> {
    specs(model).into_iter().map(|spec| spec.to_dto()).collect()
}

/// 生成共享工具规格响应体。
pub fn list_tool_specs_response(model: Option<&str>) -> ListToolSpecsResponse {
    ListToolSpecsResponse { items: spec_dtos(model) }
}

/// 将过滤后的工具规格列表转换为 JSON 数组格式
///
/// 此函数是对 `specs()` 函数的 JSON 序列化包装，返回一个 JSON 数组，
/// 其中每个元素都是一个包含工具 id、description 和 parameters 的 JSON 对象。
///
/// # 参数
///
/// - `model`: 可选的模型标识符字符串，传递给 `specs()` 函数进行过滤
///
/// # 返回值
///
/// 返回一个 `serde_json::Value::Array`，包含所有可用工具的 JSON 表示
///
/// # JSON 结构
///
/// 每个工具的 JSON 对象结构如下：
/// ```json
/// {
///   "id": "tool_id",
///   "description": "工具描述",
///   "parameters": { /* JSON Schema 格式的参数定义 */ }
/// }
/// ```
///
/// # 示例
///
/// ```ignore
/// use crate::app::agent::tools::registry::specs_json;
///
/// // 获取所有工具的 JSON 表示
/// let all_tools_json = specs_json(None);
///
/// // 获取特定模型的工具 JSON
/// let tools_json = specs_json(Some("openai/gpt-4"));
///
/// // 结果示例
/// // [
/// //   {"id": "shell", "description": "...", "parameters": {...}},
/// //   {"id": "read", "description": "...", "parameters": {...}},
/// //   ...
/// // ]
/// ```
pub fn specs_json(model: Option<&str>) -> serde_json::Value {
    // 获取过滤后的工具规格列表，并转换为 JSON 数组
    serde_json::Value::Array(
        specs(model)
            .into_iter()
            .map(|s| {
                // 将每个工具规格转换为 JSON 对象
                json!({
                    "id": s.id,
                    "description": s.description,
                    "parameters": s.input_schema
                })
            })
            .collect(),
    )
}

/// 生成符合 OpenAI 函数调用格式的工具 JSON
///
/// 此函数生成 OpenAI API 兼容的工具定义格式，适用于 OpenAI 的 Chat Completions API
/// 和 Assistants API。工具列表会按工具 ID 字母顺序排序以确保输出的一致性。
///
/// # 参数
///
/// - `model`: 可选的模型标识符字符串，传递给 `specs()` 函数进行过滤
///
/// # 返回值
///
/// 返回一个 `serde_json::Value::Array`，包含符合 OpenAI 函数调用格式的工具定义
///
/// # OpenAI 工具格式
///
/// 每个工具的 JSON 对象遵循 OpenAI 的函数调用规范：
/// ```json
/// {
///   "type": "function",
///   "function": {
///     "name": "tool_id",
///     "description": "工具描述",
///     "parameters": { /* JSON Schema 格式的参数定义 */ }
///   }
/// }
/// ```
///
/// # 与 specs_json 的区别
///
/// - `specs_json()`: 返回简化的工具 JSON 格式，适用于内部使用或自定义格式
/// - `openai_tools_json()`: 返回符合 OpenAI API 规范的格式，包含 type 和 function 包装
/// - `openai_tools_json()` 会对工具列表进行排序以确保输出一致性
///
/// # 示例
///
/// ```ignore
/// use crate::app::agent::tools::registry::openai_tools_json;
///
/// // 生成 OpenAI 兼容的工具定义
/// let tools = openai_tools_json(Some("openai/gpt-4"));
///
/// // 结果示例
/// // [
/// //   {
/// //     "type": "function",
/// //     "function": {
/// //       "name": "shell",
/// //       "description": "执行 Shell 命令",
/// //       "parameters": {...}
/// //     }
/// //   },
/// //   ...
/// // ]
/// ```
pub fn openai_tools_json(model: Option<&str>) -> serde_json::Value {
    // 获取过滤后的工具规格列表
    let mut specs = specs(model);

    // 按工具 ID 排序以确保输出的一致性和可预测性
    specs.sort_by(|a, b| a.id.cmp(&b.id));

    // 转换为 OpenAI 函数调用格式
    serde_json::Value::Array(
        specs
            .into_iter()
            .map(|s| {
                json!({
                    "type": "function",
                    "function": {
                        "name": s.id,
                        "description": s.description,
                        "parameters": s.input_schema
                    }
                })
            })
            .collect(),
    )
}
#[cfg(test)]
mod tests;
