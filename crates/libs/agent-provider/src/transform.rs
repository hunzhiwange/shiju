//! # Provider 消息转换模块
//!
//! 本模块提供了 AI 模型提供商消息格式转换的工具函数。
//! 主要用于在不同提供商的 API 格式之间进行标准化和适配。
//!
//! ## 核心功能
//!
//! - **消息标准化**：将不同提供商的消息格式统一为标准格式
//! - **选项重映射**：在不同提供商的配置选项键名之间进行转换
//! - **Token 限制管理**：处理输出 token 数量限制
//!
//! ## 使用场景
//!
//! 当需要将消息发送给不同的 AI 模型提供商时，使用本模块的函数
//! 可以确保消息格式符合目标提供商的 API 要求。

use runtime_flags as flag;
use serde_json::{Map, Value};

/// 默认的最大输出 token 数量
///
/// 当未通过环境变量 `VIBEWINDOW_EXPERIMENTAL_OUTPUT_TOKEN_MAX` 配置时，
/// 使用此默认值作为输出 token 的上限。
pub const OUTPUT_TOKEN_MAX_DEFAULT: u64 = 32_000;

/// 获取配置的输出 token 最大值
///
/// 优先从实验性环境变量 `VIBEWINDOW_EXPERIMENTAL_OUTPUT_TOKEN_MAX` 读取，
/// 如果未设置则返回默认值。
///
/// # 返回值
///
/// 返回配置的最大输出 token 数量
fn output_token_max() -> u64 {
    flag::VIBEWINDOW_EXPERIMENTAL_OUTPUT_TOKEN_MAX.unwrap_or(OUTPUT_TOKEN_MAX_DEFAULT)
}

/// 检查 JSON 值是否为空字符串
///
/// # 参数
///
/// - `v`: 要检查的 JSON 值引用
///
/// # 返回值
///
/// 如果值是字符串类型且为空，返回 `true`；否则返回 `false`
fn is_empty_string(v: &Value) -> bool {
    v.as_str().is_some_and(|s| s.is_empty())
}

/// 重映射 provider 选项的键名
///
/// 将 JSON 对象中的某个键重命名为另一个键，用于在不同提供商的
/// 配置格式之间进行转换。
///
/// # 参数
///
/// - `opts`: 包含 provider 选项的 JSON 值
/// - `from`: 要重命名的原始键名
/// - `to`: 新的目标键名
///
/// # 返回值
///
/// 返回键名重映射后的 JSON 值。如果输入不是对象或不包含原始键，
/// 则返回原始值的克隆。
///
/// # 示例
///
/// ```ignore
/// let opts = json!({"openai": {"temperature": 0.7}});
/// let remapped = remap_provider_options(&opts, "openai", "azure");
/// // 结果: {"azure": {"temperature": 0.7}}
/// ```
fn remap_provider_options(opts: &Value, from: &str, to: &str) -> Value {
    let Some(obj) = opts.as_object() else {
        return opts.clone();
    };
    if !obj.contains_key(from) {
        return opts.clone();
    };
    let mut out = obj.clone();
    if let Some(v) = out.remove(from) {
        out.insert(to.to_string(), v);
    }
    Value::Object(out)
}

/// 标准化消息列表以适配目标模型提供商
///
/// 根据不同的模型适配器和 API ID，对消息列表进行格式转换和清理，
/// 确保消息格式符合目标提供商的要求。
///
/// # 主要处理逻辑
///
/// 1. **Anthropic 适配器处理**：
///    - 过滤掉内容为空字符串的消息
///    - 过滤掉文本内容为空的 text/reasoning 类型内容块
///    - 如果过滤后内容数组为空，则移除整条消息
///
/// 2. **Claude 模型处理**：
///    - 标准化 tool-call 和 tool-result 内容块中的 toolCallId
///    - 将非字母数字字符（除下划线和连字符外）替换为下划线
///
/// # 参数
///
/// - `msgs`: 原始消息列表，每条消息为 JSON 值
/// - `model_adapter`: 模型适配器名称（如 "anthropic"、"openai" 等）
/// - `model_api_id`: 模型 API 标识符（用于判断具体模型类型，如 "claude-3-opus"）
///
/// # 返回值
///
/// 返回经过标准化处理后的消息列表
///
/// # 示例
///
/// ```ignore
/// let messages = vec![
///     json!({"role": "user", "content": ""}),  // 空消息将被过滤
///     json!({"role": "assistant", "content": [{"type": "text", "text": "Hello"}]})
/// ];
/// let normalized = normalize_messages(messages, "anthropic", "claude-3-opus");
/// ```
pub fn normalize_messages(
    mut msgs: Vec<Value>,
    model_adapter: &str,
    model_api_id: &str,
) -> Vec<Value> {
    let adapter = normalize_adapter(model_adapter);

    // 针对 Anthropic 适配器进行消息清理
    if adapter == "anthropic" || adapter.ends_with("/anthropic") {
        msgs = msgs
            .into_iter()
            .filter_map(|mut msg| {
                if let Some(content) = msg.get("content") {
                    // 过滤掉空字符串内容
                    if is_empty_string(content) {
                        return None;
                    }
                    // 处理内容数组，过滤空文本块
                    if let Some(arr) = content.as_array() {
                        let filtered = arr
                            .iter()
                            .filter(|part| {
                                // 对于 text 和 reasoning 类型，检查文本是否非空
                                if part
                                    .get("type")
                                    .and_then(Value::as_str)
                                    .is_some_and(|t| t == "text" || t == "reasoning")
                                {
                                    return part
                                        .get("text")
                                        .and_then(Value::as_str)
                                        .is_some_and(|s| !s.is_empty());
                                }
                                true
                            })
                            .cloned()
                            .collect::<Vec<_>>();
                        // 如果过滤后数组为空，移除整条消息
                        if filtered.is_empty() {
                            return None;
                        }
                        if let Some(obj) = msg.as_object_mut() {
                            obj.insert("content".to_string(), Value::Array(filtered));
                        }
                    }
                }
                Some(msg)
            })
            .collect();
    }

    // 针对 Claude 模型标准化 toolCallId
    if model_api_id.contains("claude") {
        for msg in &mut msgs {
            let Some(obj) = msg.as_object_mut() else { continue };
            let role = obj.get("role").and_then(Value::as_str).unwrap_or_default();
            // 仅处理 assistant 和 tool 角色的消息
            if role != "assistant" && role != "tool" {
                continue;
            }
            let Some(arr) = obj.get_mut("content").and_then(Value::as_array_mut) else { continue };
            for part in arr.iter_mut() {
                let Some(p) = part.as_object_mut() else { continue };
                let ty = p.get("type").and_then(Value::as_str).unwrap_or_default();
                // 标准化 tool-call 和 tool-result 的 toolCallId
                if (ty == "tool-call" || ty == "tool-result") && p.contains_key("toolCallId") {
                    if let Some(id) = p.get("toolCallId").and_then(Value::as_str) {
                        // 将非字母数字字符（除 _ 和 -）替换为下划线
                        let normalized = id
                            .chars()
                            .map(|c| {
                                if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                                    c
                                } else {
                                    '_'
                                }
                            })
                            .collect::<String>();
                        p.insert("toolCallId".to_string(), Value::String(normalized));
                    }
                }
            }
        }
    }

    msgs
}

/// 应用 provider 选项键名重映射到消息列表
///
/// 遍历消息列表，将每条消息及其内容块中的 `providerOptions` 键名
/// 从原始 provider ID 转换为目标适配器的标准键名。
///
/// # 使用场景
///
/// 当消息从某个提供商发送到另一个提供商时，需要将配置选项的键名
/// 从源提供商的命名空间转换到目标提供商的命名空间。
///
/// # 参数
///
/// - `msgs`: 消息列表
/// - `model_provider_id`: 原始模型提供商标识符（作为源键名）
/// - `model_adapter`: 目标模型适配器名称
///
/// # 返回值
///
/// 返回经过选项键名重映射后的消息列表
///
/// # 跳过条件
///
/// 以下情况不进行重映射，直接返回原始消息：
/// - 适配器没有对应的标准化键名
/// - 目标键名与源 provider ID 相同
/// - 适配器为 "azure"（Azure 保持原始键名）
///
/// # 示例
///
/// ```ignore
/// let messages = vec![
///     json!({
///         "role": "user",
///         "content": "Hello",
///         "providerOptions": {"openai": {"temperature": 0.7}}
///     })
/// ];
/// let remapped = apply_provider_options_key_remap(
///     messages,
///     "openai",
///     "azure"
/// );
/// // Azure 适配器不重映射，保持原始键名
/// ```
pub fn apply_provider_options_key_remap(
    msgs: Vec<Value>,
    model_provider_id: &str,
    model_adapter: &str,
) -> Vec<Value> {
    let key = adapter_key(model_adapter);
    let Some(key) = key else {
        return msgs;
    };
    // 如果目标键与源键相同，或适配器为 Azure，则无需重映射
    if key == model_provider_id || normalize_adapter(model_adapter) == "azure" {
        return msgs;
    }
    msgs.into_iter()
        .map(|mut msg| {
            if let Some(obj) = msg.as_object_mut() {
                // 重映射消息级别的 providerOptions
                if let Some(opts) = obj.get("providerOptions") {
                    let remapped = remap_provider_options(opts, model_provider_id, key);
                    obj.insert("providerOptions".to_string(), remapped);
                }
                // 重映射内容块级别的 providerOptions
                if let Some(arr) = obj.get_mut("content").and_then(Value::as_array_mut) {
                    for part in arr.iter_mut() {
                        let Some(p) = part.as_object_mut() else { continue };
                        if let Some(opts) = p.get("providerOptions") {
                            let remapped = remap_provider_options(opts, model_provider_id, key);
                            p.insert("providerOptions".to_string(), remapped);
                        }
                    }
                }
            }
            msg
        })
        .collect()
}

/// 计算实际的最大输出 token 数量
///
/// 取模型限制和全局配置的最小值，确保不超过任一限制。
/// 如果结果为 0，则返回全局配置值。
///
/// # 参数
///
/// - `model_limit_output`: 模型本身支持的最大输出 token 数量
///
/// # 返回值
///
/// 返回实际可用的最大输出 token 数量
///
/// # 示例
///
/// ```ignore
/// // 假设全局配置为 32000
/// assert_eq!(max_output_tokens(40000), 32000);  // 取较小值
/// assert_eq!(max_output_tokens(16000), 16000);  // 取较小值
/// assert_eq!(max_output_tokens(0), 32000);      // 0 时使用全局配置
/// ```
pub fn max_output_tokens(model_limit_output: u64) -> u64 {
    let max = output_token_max();
    let v = model_limit_output.min(max);
    if v == 0 { max } else { v }
}

/// 标准化适配器名称
///
/// 去除字符串首尾的空白字符，返回标准化的适配器名称引用。
///
/// # 参数
///
/// - `s`: 原始适配器名称字符串
///
/// # 返回值
///
/// 返回去除空白后的字符串切片
fn normalize_adapter(s: &str) -> &str {
    match s.trim() {
        "acp" | "agent-client-protocol" | "agent_client_protocol" => "openai-compatible",
        other => other,
    }
}

/// 获取适配器对应的标准化键名
///
/// 将各种适配器名称映射到标准的 provider 键名，用于配置选项的命名空间。
///
/// # 参数
///
/// - `adapter`: 适配器名称（如 "openai"、"anthropic"、"github-copilot" 等）
///
/// # 返回值
///
/// 返回对应的标准化键名，如果适配器未识别则返回 `None`
///
/// # 键名映射表
///
/// | 适配器名称 | 标准键名 |
/// |-----------|---------|
/// | github-copilot | copilot |
/// | openai, openai-compatible, azure | openai |
/// | amazon-bedrock, bedrock | bedrock |
/// | anthropic, google-vertex/anthropic | anthropic |
/// | google-vertex, google | google |
/// | gateway | gateway |
/// | openrouter | openrouter |
fn adapter_key(adapter: &str) -> Option<&'static str> {
    match normalize_adapter(adapter) {
        "github-copilot" => Some("copilot"),
        "openai"
        | "openai-compatible"
        | "azure"
        | "acp"
        | "agent-client-protocol"
        | "agent_client_protocol" => Some("openai"),
        "amazon-bedrock" | "bedrock" => Some("bedrock"),
        "anthropic" | "google-vertex/anthropic" => Some("anthropic"),
        "google-vertex" | "google" => Some("google"),
        "gateway" => Some("gateway"),
        "openrouter" => Some("openrouter"),
        _ => None,
    }
}

/// 构造 provider 选项对象
///
/// 将原始选项包装在以 provider 键名为命名空间的对象中，
/// 确保选项与特定的提供商关联。
///
/// # 参数
///
/// - `model_provider_id`: 模型提供商标识符（作为备选键名）
/// - `model_adapter`: 模型适配器名称
/// - `options`: 原始选项配置（JSON 值）
///
/// # 返回值
///
/// 返回包装后的 JSON 对象，格式为 `{ "<provider_key>": <options> }`
///
/// # 示例
///
/// ```ignore
/// let options = json!({"temperature": 0.7, "max_tokens": 1000});
/// let wrapped = provider_options("openai", "openai", options);
/// // 结果: {"openai": {"temperature": 0.7, "max_tokens": 1000}}
/// ```
pub fn provider_options(model_provider_id: &str, model_adapter: &str, options: Value) -> Value {
    let key = adapter_key(model_adapter).unwrap_or(model_provider_id);
    if options.as_object().is_some_and(|object| object.contains_key(key)) {
        return options;
    }
    let mut out = Map::new();
    out.insert(key.to_string(), options);
    Value::Object(out)
}

#[cfg(test)]
#[path = "transform_tests.rs"]
mod transform_tests;
