//! # 工具调用格式解析模块
//!
//! 本模块提供多种 LLM 提供商输出的工具调用格式解析能力。
//! 由于不同 AI 提供商（如 OpenAI、GLM、MiniMax 等）可能使用
//! 不同的工具调用语法，此模块实现了统一的解析接口，
//! 将各种格式规范化为标准的 [`ParsedToolCall`] 结构。
//!
//! ## 支持的格式
//!
//! 1. **XML 属性风格** - MiniMax 等提供商使用：
//!    ```xml
//!    <minimax:toolcall>
//!      <invoke name="bash">
//!        <parameter name="command">ls</parameter>
//!      </invoke>
//!    </minimax:toolcall>
//!    ```
//!
//! 2. **Perl/哈希引用风格**：
//!    ```text
//!    TOOL_CALL
//!    {tool => "bash", args => {
//!      --command "ls -la"
//!    }}
//!    /TOOL_CALL
//!    ```
//!
//! 3. **FunctionCall 风格**：
//!    ```text
//!    <FunctionCall>
//!    file_read
//!    <code>path>/Users/.../README.md</code>
//!    </FunctionCall>
//!    ```
//!
//! 4. **GLM 缩写风格** - GLM-4.x 系列使用：
//!    - 单值格式：`bash>ls -la`
//!    - 参数格式：`bash/command>ls -la`
//!    - YAML 多行格式
//!    - 属性风格：`bash command="ls -la" />`

use crate::app::agent::agent::loop_::parsing::ParsedToolCall;
use regex::Regex;
use std::sync::LazyLock;

#[cfg(test)]
#[path = "tool_call_formats_tests.rs"]
mod tool_call_formats_tests;

/// 从原始 JSON 值中提取字符串参数提示。
///
/// 该函数用于从可能包含原始字符串参数的 JSON 值中
/// 提取非空的字符串内容，作为后续规范化处理的提示信息。
///
/// # 参数
///
/// * `raw` - 可选的 JSON 值引用，预期为字符串类型
///
/// # 返回值
///
/// 如果输入存在、是字符串类型、非空且去除空白后仍非空，
/// 则返回 `Some(&str)`，否则返回 `None`。
///
/// # 示例
///
/// ```
/// use serde_json::json;
/// let value = json!("  hello  ");
/// assert_eq!(raw_string_argument_hint(Some(&value)), Some("hello"));
///
/// let empty = json!("   ");
/// assert_eq!(raw_string_argument_hint(Some(&empty)), None);
/// ```
pub(crate) fn raw_string_argument_hint(raw: Option<&serde_json::Value>) -> Option<&str> {
    raw.and_then(|value| value.as_str()).map(str::trim).filter(|s| !s.is_empty())
}

/// 构建 curl 命令字符串。
///
/// 验证 URL 的有效性并生成安全的 curl 命令。
/// 对 URL 中的单引号进行转义以防止命令注入。
///
/// # 参数
///
/// * `url` - 待请求的 URL 字符串
///
/// # 返回值
///
/// 如果 URL 有效（以 http:// 或 https:// 开头且不含空白字符），
/// 返回 `Some(String)` 包含格式化的 curl 命令；
/// 否则返回 `None`。
///
/// # 安全性
///
/// - 仅接受 http/https 协议
/// - 拒绝包含空白字符的 URL
/// - 对单引号进行转义处理
///
/// # 示例
///
/// ```
/// assert_eq!(
///     build_curl_command("https://example.com"),
///     Some("curl -s 'https://example.com'".to_string())
/// );
/// assert_eq!(build_curl_command("ftp://invalid"), None);
/// assert_eq!(build_curl_command("https://has space.com"), None);
/// ```
pub(crate) fn build_curl_command(url: &str) -> Option<String> {
    // 验证协议：仅接受 http 和 https
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return None;
    }

    // 验证安全性：URL 中不允许有空白字符
    if url.chars().any(char::is_whitespace) {
        return None;
    }

    // 转义单引号，防止 shell 注入
    let escaped = url.replace('\'', r#"'\\''"#);
    Some(format!("curl -s '{}'", escaped))
}

/// 规范化原始 shell 命令字符串。
///
/// 对原始命令字符串进行清理和验证，去除引号包裹、
/// 检测并拒绝 JSON 对象/数组，并将 URL 转换为 curl 命令。
///
/// # 参数
///
/// * `raw` - 原始命令字符串
///
/// # 返回值
///
/// 返回规范化后的命令字符串，或在以下情况返回 `None`：
/// - 输入为空或仅包含空白
/// - 内容看起来像 JSON 对象或数组
/// - URL 格式无效
///
/// # 处理逻辑
///
/// 1. 去除首尾空白
/// 2. 去除双引号或单引号包裹
/// 3. 检测并拒绝 JSON 结构（以 `{` 或 `[` 开头）
/// 4. 如果是 URL，转换为 curl 命令
///
/// # 示例
///
/// ```
/// assert_eq!(normalize_shell_command_from_raw("  \"ls -la\"  "), Some("ls -la".to_string()));
/// assert_eq!(normalize_shell_command_from_raw("https://api.example.com"), Some("curl -s 'https://api.example.com'".to_string()));
/// assert_eq!(normalize_shell_command_from_raw("{\"key\": \"value\"}"), None);
/// ```
pub(crate) fn normalize_shell_command_from_raw(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    // 去除引号包裹（支持双引号和单引号）
    let unwrapped = trimmed
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| trimmed.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
        .unwrap_or(trimmed)
        .trim();

    if unwrapped.is_empty() {
        return None;
    }

    // 拒绝 JSON 对象和数组（可能是误解析的结果）
    if (unwrapped.starts_with('{') && unwrapped.ends_with('}'))
        || (unwrapped.starts_with('[') && unwrapped.ends_with(']'))
    {
        return None;
    }

    // 如果是 URL，构建 curl 命令
    if unwrapped.starts_with("http://") || unwrapped.starts_with("https://") {
        return build_curl_command(unwrapped).or_else(|| Some(unwrapped.to_string()));
    }

    Some(unwrapped.to_string())
}

/// 规范化 shell 工具的参数。
///
/// 将各种可能的参数格式统一转换为包含 `command` 字段的对象。
/// 支持多种命令字段的别名，并能处理 URL 参数。
///
/// # 参数
///
/// * `arguments` - 原始参数值，可以是对象、字符串或其他类型
/// * `raw_string_hint` - 从原始响应中提取的字符串提示
///
/// # 返回值
///
/// 返回规范化后的 JSON 对象，至少包含 `command` 字段。
/// 如果无法提取有效命令，返回空对象。
///
/// # 支持的命令字段别名
///
/// - `cmd`
/// - `script`
/// - `shell_command`
/// - `command_line`
/// - `bash`
/// - `sh`
/// - `input`
///
/// # 示例
///
/// ```ignore
/// use serde_json::json;
///
/// // 从 cmd 别名提取
/// let args = json!({"cmd": "ls"});
/// let normalized = normalize_shell_arguments(args, None);
/// assert_eq!(normalized["command"], "ls");
///
/// // 从字符串直接转换
/// let args = json!("ls -la");
/// let normalized = normalize_shell_arguments(args, None);
/// assert_eq!(normalized["command"], "ls -la");
/// ```
pub(crate) fn normalize_shell_arguments(
    arguments: serde_json::Value,
    raw_string_hint: Option<&str>,
) -> serde_json::Value {
    match arguments {
        serde_json::Value::Object(mut map) => {
            // 如果已存在有效的 command 字段，直接返回
            if map
                .get("command")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .is_some_and(|cmd| !cmd.is_empty())
            {
                return serde_json::Value::Object(map);
            }

            // 尝试从各种别名字段提取命令
            for alias in ["cmd", "script", "shell_command", "command_line", "bash", "sh", "input"] {
                if let Some(value) = map.get(alias).and_then(|v| v.as_str()) {
                    if let Some(command) = normalize_shell_command_from_raw(value) {
                        map.insert("command".to_string(), serde_json::Value::String(command));
                        return serde_json::Value::Object(map);
                    }
                }
            }

            // 尝试从 url 或 http_url 字段提取并构建 curl 命令
            if let Some(url) = map
                .get("url")
                .or_else(|| map.get("http_url"))
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|url| !url.is_empty())
            {
                if let Some(command) = normalize_shell_command_from_raw(url) {
                    map.insert("command".to_string(), serde_json::Value::String(command));
                    return serde_json::Value::Object(map);
                }
            }

            // 最后尝试使用原始字符串提示
            if let Some(raw) = raw_string_hint.and_then(normalize_shell_command_from_raw) {
                map.insert("command".to_string(), serde_json::Value::String(raw));
            }

            serde_json::Value::Object(map)
        }
        // 字符串参数：直接作为命令处理
        serde_json::Value::String(raw) => normalize_shell_command_from_raw(&raw)
            .map(|command| serde_json::json!({ "command": command }))
            .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new())),
        // 其他类型：尝试使用原始字符串提示
        _ => raw_string_hint
            .and_then(normalize_shell_command_from_raw)
            .map(|command| serde_json::json!({ "command": command }))
            .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new())),
    }
}

fn single_string_argument(key: &str, value: &str) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    map.insert(key.to_string(), serde_json::Value::String(value.to_string()));
    serde_json::Value::Object(map)
}

fn normalize_text_arguments(
    arguments: serde_json::Value,
    raw_string_hint: Option<&str>,
    primary_key: &str,
    alias_keys: &[&str],
) -> serde_json::Value {
    let hinted = raw_string_hint.map(str::trim).filter(|value| !value.is_empty());

    let extract = |map: &serde_json::Map<String, serde_json::Value>, key: &str| {
        map.get(key)
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string())
    };

    match arguments {
        serde_json::Value::Object(mut map) => {
            if extract(&map, primary_key).is_some() {
                return serde_json::Value::Object(map);
            }

            for alias in alias_keys {
                if let Some(value) = extract(&map, alias) {
                    map.insert(primary_key.to_string(), serde_json::Value::String(value));
                    return serde_json::Value::Object(map);
                }
            }

            if let Some(value) = hinted {
                map.insert(primary_key.to_string(), serde_json::Value::String(value.to_string()));
            }

            serde_json::Value::Object(map)
        }
        serde_json::Value::String(raw) => {
            let raw = raw.trim();
            if raw.is_empty() {
                serde_json::Value::Object(serde_json::Map::new())
            } else {
                single_string_argument(primary_key, raw)
            }
        }
        _ => hinted
            .map(|value| single_string_argument(primary_key, value))
            .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new())),
    }
}

/// 根据工具名称规范化工具参数。
///
/// 根据不同的工具类型应用相应的参数规范化逻辑。
/// 目前处理内部 shell 命令工具，以及当前搜索工具与历史搜索别名的特殊格式。
///
/// # 参数
///
/// * `tool_name` - 工具名称
/// * `arguments` - 原始参数值
/// * `raw_string_hint` - 从原始响应中提取的字符串提示
///
/// # 返回值
///
/// 返回针对特定工具类型规范化后的参数。
///
/// # 示例
///
/// ```ignore
/// use serde_json::json;
///
/// let args = json!({"cmd": "ls"});
/// let normalized = normalize_tool_arguments("shell", args, None);
/// assert_eq!(normalized["command"], "ls");
/// ```
pub(crate) fn normalize_tool_arguments(
    tool_name: &str,
    arguments: serde_json::Value,
    raw_string_hint: Option<&str>,
) -> serde_json::Value {
    match canonicalize_tool_name(tool_name) {
        "shell" => normalize_shell_arguments(arguments, raw_string_hint),
        "grep" | "glob" => {
            normalize_text_arguments(arguments, raw_string_hint, "pattern", &["input", "query"])
        }
        "codesearch" => {
            normalize_text_arguments(arguments, raw_string_hint, "query", &["input", "pattern"])
        }
        _ => arguments,
    }
}

/// 从响应文本中解析 XML 属性风格的工具调用。
///
/// 处理 MiniMax 及类似提供商输出的格式：
/// ```xml
/// <minimax:toolcall>
///   <invoke name="shell">
///     <parameter name="command">ls</parameter>
///   </invoke>
/// </minimax:toolcall>
/// ```
///
/// # 参数
///
/// * `response` - LLM 响应文本
///
/// # 返回值
///
/// 返回解析出的工具调用列表。每个调用包含工具名称和参数对象。
///
/// # 正则表达式模式
///
/// - `INVOKE_RE`：匹配 `<invoke name="...">...</invoke>` 块
/// - `PARAM_RE`：匹配 `<parameter name="...">value</parameter>`
pub(crate) fn parse_xml_attribute_tool_calls(response: &str) -> Vec<ParsedToolCall> {
    let mut calls = Vec::new();

    // 匹配 <invoke name="toolname">...</invoke> 块的正则表达式
    static INVOKE_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?s)<invoke\s+name=\"([^\"]+)\"[^>]*>(.*?)</invoke>"#).unwrap()
    });

    // 匹配 <parameter name="paramname">value</parameter> 的正则表达式
    static PARAM_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"<parameter\s+name=\"([^\"]+)\"[^>]*>([^<]*)</parameter>"#).unwrap()
    });

    for cap in INVOKE_RE.captures_iter(response) {
        let tool_name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let inner = cap.get(2).map(|m| m.as_str()).unwrap_or("");

        let mut arguments = serde_json::Map::new();

        // 解析所有参数
        for param_cap in PARAM_RE.captures_iter(inner) {
            let param_name = param_cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let param_value = param_cap.get(2).map(|m| m.as_str()).unwrap_or("");

            if !param_name.is_empty() {
                arguments.insert(
                    param_name.to_string(),
                    serde_json::Value::String(param_value.to_string()),
                );
            }
        }

        if !arguments.is_empty() {
            calls.push(ParsedToolCall {
                name: canonicalize_tool_name(tool_name).to_string(),
                arguments: serde_json::Value::Object(arguments),
                tool_call_id: None,
            });
        }
    }

    calls
}

/// 从响应文本中解析 Perl/哈希引用风格的工具调用。
///
/// 处理如下格式：
/// ```text
/// TOOL_CALL
/// {tool => "shell", args => {
///   --command "ls -la"
///   --description "List current directory contents"
/// }}
/// /TOOL_CALL
/// ```
///
/// # 参数
///
/// * `response` - LLM 响应文本
///
/// # 返回值
///
/// 返回解析出的工具调用列表。
///
/// # 正则表达式模式
///
/// - `PERL_RE`：匹配 `TOOL_CALL {...}} /TOOL_CALL` 块
/// - `TOOL_NAME_RE`：匹配 `tool => "name"`
/// - `ARGS_BLOCK_RE`：匹配 `args => { ... }` 块
/// - `ARGS_RE`：匹配 `--key "value"` 参数对
pub(crate) fn parse_perl_style_tool_calls(response: &str) -> Vec<ParsedToolCall> {
    let mut calls = Vec::new();

    // 匹配 TOOL_CALL 块的正则（处理双重闭合大括号 }}）
    static PERL_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?s)TOOL_CALL\s*\{(.+?)\}\}\s*/TOOL_CALL").unwrap());

    // 匹配 tool => "name" 的正则
    static TOOL_NAME_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"tool\s*=>\s*\"([^\"]+)\""#).unwrap());

    // 匹配 args => { ... } 块的正则
    static ARGS_BLOCK_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?s)args\s*=>\s*\{(.+?)\}").unwrap());

    // 匹配 --key "value" 参数对的正则
    static ARGS_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"--(\w+)\s+\"([^\"]+)\""#).unwrap());

    for cap in PERL_RE.captures_iter(response) {
        let content = cap.get(1).map(|m| m.as_str()).unwrap_or("");

        // 提取工具名称
        let tool_name =
            TOOL_NAME_RE.captures(content).and_then(|c| c.get(1)).map(|m| m.as_str()).unwrap_or("");

        if tool_name.is_empty() {
            continue;
        }

        // 提取参数块
        let args_block = ARGS_BLOCK_RE
            .captures(content)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str())
            .unwrap_or("");

        let mut arguments = serde_json::Map::new();

        // 解析所有 --key "value" 参数
        for arg_cap in ARGS_RE.captures_iter(args_block) {
            let key = arg_cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let value = arg_cap.get(2).map(|m| m.as_str()).unwrap_or("");

            if !key.is_empty() {
                arguments.insert(key.to_string(), serde_json::Value::String(value.to_string()));
            }
        }

        if !arguments.is_empty() {
            calls.push(ParsedToolCall {
                name: canonicalize_tool_name(tool_name).to_string(),
                arguments: serde_json::Value::Object(arguments),
                tool_call_id: None,
            });
        }
    }

    calls
}

/// 从响应文本中解析 FunctionCall 风格的工具调用。
///
/// 处理如下格式：
/// ```text
/// <FunctionCall>
/// file_read
/// <code>path>/Users/kylelampa/Documents/vibewindow/README.md</code>
/// </FunctionCall>
/// ```
///
/// # 参数
///
/// * `response` - LLM 响应文本
///
/// # 返回值
///
/// 返回解析出的工具调用列表。参数以 `key>value` 格式解析。
///
/// # 正则表达式模式
///
/// - `FUNC_RE`：匹配 `<FunctionCall>toolname<code>args</code></FunctionCall>` 块
pub(crate) fn parse_function_call_tool_calls(response: &str) -> Vec<ParsedToolCall> {
    let mut calls = Vec::new();

    // 匹配 <FunctionCall> 块的正则
    static FUNC_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?s)<FunctionCall>\s*(\w+)\s*<code>([^<]+)</code>\s*</FunctionCall>").unwrap()
    });

    for cap in FUNC_RE.captures_iter(response) {
        let tool_name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let args_text = cap.get(2).map(|m| m.as_str()).unwrap_or("");

        // 解析 key>value 参数对（例如 path>/Users/.../file.txt）
        let mut arguments = serde_json::Map::new();
        for line in args_text.lines() {
            let line = line.trim();
            if let Some(pos) = line.find('>') {
                let key = line[..pos].trim();
                let value = line[pos + 1..].trim();
                if !key.is_empty() && !value.is_empty() {
                    arguments.insert(key.to_string(), serde_json::Value::String(value.to_string()));
                }
            }
        }

        if !arguments.is_empty() {
            calls.push(ParsedToolCall {
                name: canonicalize_tool_name(tool_name).to_string(),
                arguments: serde_json::Value::Object(arguments),
                tool_call_id: None,
            });
        }
    }

    calls
}

/// 将提示协议里的历史工具名称折叠为运行时使用的工具名称。
///
/// 不同 LLM 提供商可能使用不同的工具命名约定。
/// 此函数将各种变体统一映射到 VibeWindow 的标准工具名称。
///
/// # 参数
///
/// * `tool_name` - 原始工具名称（可能来自不同提供商）
///
/// # 返回值
///
/// 返回映射后的标准工具名称。如果未找到映射，返回原名称。
///
/// # 支持的映射
///
/// | 类别 | 别名 | 标准名称 |
/// |------|------|----------|
/// | 命令 | shell, bash, sh, exec, command, cmd | shell |
/// | 消息 | send_message, sendmessage | message_send |
/// | 文件读取 | fileread, file_read, readfile, read_file, file | file_read |
/// | 文件编辑 | edit, file_edit, editfile, edit_file | edit |
/// | 文件写入 | write, filewrite, file_write, writefile, write_file | file_write |
/// | 文件列表 | filelist, file_list, listfiles, list_files | file_list |
/// | 记忆召回 | memoryrecall, memory_recall, recall, memrecall | memory_recall |
/// | 记忆存储 | memorystore, memory_store, store, memstore | memory_store |
/// | 记忆删除 | memoryforget, memory_forget, forget, memforget | memory_forget |
/// | HTTP | http_request, http, fetch, curl, wget | http_request |
///
/// # 示例
///
/// ```
/// assert_eq!(canonicalize_tool_name("bash"), "shell");
/// assert_eq!(canonicalize_tool_name("fileread"), "file_read");
/// assert_eq!(canonicalize_tool_name("unknown_tool"), "unknown_tool");
/// ```
pub(crate) fn canonicalize_tool_name(tool_name: &str) -> &str {
    let normalized = tool_name.to_ascii_lowercase();
    match normalized.as_str() {
        // Shell 工具变体（包括映射到 shell 的 GLM 别名）
        "shell" | "bash" | "sh" | "exec" | "command" | "cmd" | "browser_open" => "shell",
        // 消息工具变体
        "send_message" | "sendmessage" => "message_send",
        // 文件工具变体
        "fileread" | "file_read" | "readfile" | "read_file" | "file" => "file_read",
        "file_edit" | "editfile" | "edit_file" => "file_edit",
        "write" | "filewrite" | "file_write" | "writefile" | "write_file" => "file_write",
        "filelist" | "file_list" | "listfiles" | "list_files" => "file_list",
        // 记忆工具变体
        "memoryrecall" | "memory_recall" | "recall" | "memrecall" => "memory_recall",
        "memorystore" | "memory_store" | "store" | "memstore" => "memory_store",
        "memoryforget" | "memory_forget" | "forget" | "memforget" => "memory_forget",
        // HTTP 工具变体
        "http_request" | "http" | "fetch" | "curl" | "wget" => "http_request",
        _ => tool_name,
    }
}

/// 从文本中解析 GLM 风格的工具调用。
///
/// GLM 模型使用特定的工具调用语法，格式包括：
/// - 参数格式：`tool_name/param>value`
/// - JSON 格式：`tool_name/{json}`
/// - 纯 URL：自动转换为命令工具 curl 调用
///
/// # 参数
///
/// * `text` - LLM 响应文本
///
/// # 返回值
///
/// 返回元组列表 `(工具名称, 参数对象, 原始行)`。
/// 原始行用于调试和追踪。
///
/// # 格式说明
///
/// 1. `bash/command>ls -la` → `("shell", {"command": "ls -la"}, ...)`
/// 2. `file_read/path>/etc/hosts` → `("file_read", {"path": "/etc/hosts"}, ...)`
/// 3. `https://api.example.com` → `("shell", {"command": "curl -s '...'"}, ...)`
pub(crate) fn parse_glm_style_tool_calls(
    text: &str,
) -> Vec<(String, serde_json::Value, Option<String>)> {
    let mut calls = Vec::new();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // 处理纯 URL：转换为 shell curl 命令
        if let Some(command) = build_curl_command(line) {
            calls.push((
                "shell".to_string(),
                serde_json::json!({ "command": command }),
                Some(line.to_string()),
            ));
            continue;
        }

        // 格式：tool_name/param>value 或 tool_name/{json}
        if let Some(pos) = line.find('/') {
            let tool_part = &line[..pos];
            let rest = &line[pos + 1..];

            // 验证工具名称格式：仅允许字母数字和下划线
            if !tool_part.chars().all(|c| c.is_alphanumeric() || c == '_') {
                continue;
            }

            let tool_name = canonicalize_tool_name(tool_part);

            // 处理 param>value 格式
            if let Some(gt_pos) = rest.find('>') {
                let param_name = rest[..gt_pos].trim();
                let value = rest[gt_pos + 1..].trim();

                let arguments = match tool_name {
                    // Shell 工具：特殊处理 URL 参数
                    "shell" => {
                        if param_name == "url" {
                            let Some(command) = build_curl_command(value) else {
                                continue;
                            };
                            serde_json::json!({ "command": command })
                        } else if value.starts_with("http://") || value.starts_with("https://") {
                            // 值本身是 URL
                            if let Some(command) = build_curl_command(value) {
                                serde_json::json!({ "command": command })
                            } else {
                                serde_json::json!({ "command": value })
                            }
                        } else {
                            serde_json::json!({ "command": value })
                        }
                    }
                    // HTTP 工具：自动添加 GET 方法
                    "http_request" => serde_json::json!({"url": value, "method": "GET"}),
                    // 其他工具：直接使用参数名和值
                    _ => serde_json::json!({ param_name: value }),
                };

                calls.push((tool_name.to_string(), arguments, Some(line.to_string())));
                continue;
            }

            // 处理 JSON 格式参数
            if !rest.starts_with('{') {
                continue;
            }
            let Ok(json_args) = serde_json::from_str::<serde_json::Value>(rest) else {
                continue;
            };
            calls.push((tool_name.to_string(), json_args, Some(line.to_string())));
        }
    }

    calls
}

/// 返回工具的默认参数名称。
///
/// 当模型输出缩写格式的工具调用（如 `bash>uname -a`，
/// 没有显式的 `/param_name`）时，需要推断值映射到哪个参数。
/// 此函数为已知的 VibeWindow 工具编码这种映射关系。
///
/// # 参数
///
/// * `tool` - 工具名称（可以是别名）
///
/// # 返回值
///
/// 返回该工具的默认参数名称。未知工具返回 `"input"`。
///
/// # 映射规则
///
/// | 工具类型 | 默认参数 |
/// |----------|----------|
/// | 命令工具 (shell, bash, sh, exec, command, cmd) | command |
/// | 文件工具 (file_read, file_write, file_list 等) | path |
/// | 记忆召回/删除 (memory_recall, memory_forget) | query |
/// | 记忆存储 (memory_store) | content |
/// | HTTP/网页工具 (http_request, BrowserOpen, WebFetch 等) | url |
/// | 搜索工具 (WebSearch, web_search_tool 等) | query |
/// | 其他 | input |
///
/// # 示例
///
/// ```
/// assert_eq!(default_param_for_tool("bash"), "command");
/// assert_eq!(default_param_for_tool("file_read"), "path");
/// assert_eq!(default_param_for_tool("memory_recall"), "query");
/// assert_eq!(default_param_for_tool("unknown"), "input");
/// ```
pub(crate) fn default_param_for_tool(tool: &str) -> &'static str {
    match tool {
        "shell" | "bash" | "sh" | "exec" | "command" | "cmd" => "command",
        // 所有文件工具默认使用 path 参数
        "file_read" | "fileread" | "readfile" | "read_file" | "file" | "write" | "file_write" | "filewrite" | "writefile"
        | "write_file" | "file_list" | "filelist" | "listfiles" | "list_files" => "path",
        "grep" | "glob" => "pattern",
        "codesearch" => "query",
        // 记忆召回和删除默认使用 query 参数
        "memory_recall" | "memoryrecall" | "recall" | "memrecall" | "memory_forget"
        | "memoryforget" | "forget" | "memforget" => "query",
        "memory_store" | "memorystore" | "store" | "memstore" => "content",
        "web_search_tool" | "websearch" | "web_search" | "WebSearch" => "query",
        // HTTP 和浏览器工具默认使用 url 参数
        "http_request" | "http" | "fetch" | "curl" | "wget" | "browser_open" | "BrowserOpen"
        | "web_fetch" | "webfetch" | "WebFetch" => "url",
        _ => "input",
    }
}

/// 解析 GLM 风格的缩写工具调用体（通常出现在 `࿠` 标签内）。
///
/// 处理 GLM-4.7 等模型输出的三种子格式：
///
/// 1. **缩写格式**：`tool_name>value` — 单值通过 [`default_param_for_tool`] 映射
/// 2. **YAML 多行格式**：`tool_name>\nkey: value\nkey: value` — 每行成为参数
/// 3. **属性风格**：`tool_name key="value" [/]>` — 类 XML 属性
///
/// # 参数
///
/// * `body` - 工具调用体文本（不含外层标记）
///
/// # 返回值
///
/// 如果成功解析，返回 `Some(ParsedToolCall)`；
/// 如果格式不匹配任何已知模式，返回 `None`。
///
/// # 解析优先级
///
/// 1. 函数调用风格：`tool_name(args)`
/// 2. 属性风格：`tool_name key="value"` — 必须先于 `>` 检查
/// 3. 缩写格式：`tool_name>value`
///
/// # 示例
///
/// ```ignore
/// // 缩写格式
/// let call = parse_glm_shortened_body("bash>ls -la");
/// assert_eq!(call.unwrap().name, "shell");
///
/// // 属性风格
/// let call = parse_glm_shortened_body("file_read path=\"/etc/hosts\" />");
/// assert_eq!(call.unwrap().arguments["path"], "/etc/hosts");
///
/// // YAML 多行格式
/// let body = "file_write>\npath: /tmp/test.txt\ncontent: hello";
/// let call = parse_glm_shortened_body(body);
/// ```
pub(crate) fn parse_glm_shortened_body(body: &str) -> Option<ParsedToolCall> {
    let body = body.trim();
    if body.is_empty() {
        return None;
    }

    // 检测函数调用风格：tool_name(args)
    let function_body = body.trim_end_matches('>').trim();
    let function_style = function_body.find('(').and_then(|open| {
        if function_body.ends_with(')') && open > 0 {
            Some((
                function_body[..open].trim(),
                function_body[open + 1..function_body.len() - 1].trim(),
            ))
        } else {
            None
        }
    });

    // 优先检查属性风格：`tool_name key="value" />`
    // 必须在 `>` 检查之前，因为 `/>` 包含 `>` 会导致第一个分支错误解析工具名
    let (tool_raw, value_part) = if let Some((tool, args)) = function_style {
        (tool, args)
    } else if body.contains("=\"") {
        // 属性风格：在第一个空白处分割以获取工具名
        let split_pos = body.find(|c: char| c.is_whitespace()).unwrap_or(body.len());
        let tool = body[..split_pos].trim();
        let attrs = body[split_pos..]
            .trim()
            .trim_end_matches("/>")
            .trim_end_matches('>')
            .trim_end_matches('/')
            .trim();
        (tool, attrs)
    } else if let Some(gt_pos) = body.find('>') {
        // GLM 缩写格式：`tool_name>value`
        let tool = body[..gt_pos].trim();
        let value = body[gt_pos + 1..].trim();
        // 去除某些模型输出的尾部自闭合标记
        let value = value.trim_end_matches("/>").trim_end_matches('/').trim();
        (tool, value)
    } else {
        return None;
    };

    // 验证工具名称：仅允许字母数字和下划线
    let tool_raw = tool_raw.trim_end_matches(|c: char| c.is_whitespace());
    if tool_raw.is_empty() || !tool_raw.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }

    let tool_name = canonicalize_tool_name(tool_raw);

    // 尝试属性风格解析：`key="value" key2="value2"`
    if value_part.contains("=\"") {
        let mut args = serde_json::Map::new();
        // 简单属性解析器：处理 key="value" 对
        let mut rest = value_part;
        while let Some(eq_pos) = rest.find("=\"") {
            // 向后查找键名的起始位置
            let key_start =
                rest[..eq_pos].rfind(|c: char| c.is_whitespace()).map(|p| p + 1).unwrap_or(0);
            let key = rest[key_start..eq_pos].trim().trim_matches(|c: char| c == ',' || c == ';');
            let after_quote = &rest[eq_pos + 2..];
            if let Some(end_quote) = after_quote.find('"') {
                let value = &after_quote[..end_quote];
                if !key.is_empty() {
                    args.insert(key.to_string(), serde_json::Value::String(value.to_string()));
                }
                rest = &after_quote[end_quote + 1..];
            } else {
                break;
            }
        }
        if !args.is_empty() {
            return Some(ParsedToolCall {
                name: tool_name.to_string(),
                arguments: serde_json::Value::Object(args),
                tool_call_id: None,
            });
        }
    }

    // 尝试 YAML 风格多行解析：每行格式为 `key: value`
    if value_part.contains('\n') {
        let mut args = serde_json::Map::new();
        for line in value_part.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(colon_pos) = line.find(':') {
                let key = line[..colon_pos].trim();
                let value = line[colon_pos + 1..].trim();
                if !key.is_empty() && !value.is_empty() {
                    // 规范化布尔值
                    let json_value = match value {
                        "true" | "yes" => serde_json::Value::Bool(true),
                        "false" | "no" => serde_json::Value::Bool(false),
                        _ => serde_json::Value::String(value.to_string()),
                    };
                    args.insert(key.to_string(), json_value);
                }
            }
        }
        if !args.is_empty() {
            return Some(ParsedToolCall {
                name: tool_name.to_string(),
                arguments: serde_json::Value::Object(args),
                tool_call_id: None,
            });
        }
    }

    // 单值缩写格式：`tool>value`
    if !value_part.is_empty() {
        let param = default_param_for_tool(tool_raw);
        let arguments = match tool_name {
            // Shell 工具：URL 自动转换为 curl 命令
            "shell" => {
                if value_part.starts_with("http://") || value_part.starts_with("https://") {
                    if let Some(cmd) = build_curl_command(value_part) {
                        serde_json::json!({ "command": cmd })
                    } else {
                        serde_json::json!({ "command": value_part })
                    }
                } else {
                    serde_json::json!({ "command": value_part })
                }
            }
            // HTTP 工具：自动添加 GET 方法
            "http_request" => serde_json::json!({"url": value_part, "method": "GET"}),
            // 其他工具：使用默认参数名
            _ => serde_json::json!({ param: value_part }),
        };
        return Some(ParsedToolCall { name: tool_name.to_string(), arguments, tool_call_id: None });
    }

    None
}
