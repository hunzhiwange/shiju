//! GLM 风格工具调用解析测试模块
//!
//! 本模块专门测试 GLM（通用语言模型）风格的工具调用格式解析功能。
//! GLM 风格是一种简化的工具调用表示法，与标准的 XML/JSON 格式不同，
//! 它使用更紧凑的语法来表示工具调用。
//!
//! # 支持的 GLM 风格格式
//!
//! 1. **简化命令格式**：`tool_name>value` 或 `tool_name/param>value`
//! 2. **JSON 参数格式**：`tool_name/{"param": "value"}`
//! 3. **URL 格式**：直接使用 URL 会被转换为 curl 命令
//! 4. **函数调用格式**：`tool_name(param="value")`
//!
//! # 测试覆盖范围
//!
//! - 基础 GLM 风格解析（bash、browser_open、http_request 等）
//! - 跨别名闭合标签测试（< ACTIONS...></invoke>）
//! - 混合格式解析（GLM 简化格式嵌入在标签中）
//! - 安全性验证（拒绝非 HTTP URL 参数）
//! - 错误处理（无效工具名、空输入等）

use crate::parsing::parse_tool_calls;
use crate::parsing::tool_call_formats::{parse_glm_shortened_body, parse_glm_style_tool_calls};

// ═══════════════════════════════════════════════════════════════════════
// GLM 风格工具调用解析测试
// ═══════════════════════════════════════════════════════════════════════

/// 测试解析 browser_open 工具调用（URL 参数）
///
/// 验证 GLM 风格的 `browser_open/url>URL` 格式能够正确解析，
/// 并转换为带有 curl 命令的命令工具调用。
///
/// # 测试场景
/// - 输入格式：`browser_open/url>https://example.com`
/// - 预期行为：生成内部 canonical 为 `shell` 的命令工具调用，命令中包含 curl 和目标 URL
#[test]
fn parse_glm_style_browser_open_url() {
    let response = "browser_open/url>https://example.com";
    let calls = parse_glm_style_tool_calls(response);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "shell");
    assert!(calls[0].1["command"].as_str().unwrap().contains("curl"));
    assert!(calls[0].1["command"].as_str().unwrap().contains("example.com"));
}

/// 测试解析 bash 命令工具调用
///
/// 验证 GLM 风格的 `bash/command>命令` 格式能够正确解析，
/// 参数会收敛到内部 `shell` 工具的 `command` 参数。
///
/// # 测试场景
/// - 输入格式：`bash/command>ls -la`
/// - 预期行为：生成内部 canonical 为 `shell` 的工具调用，command 参数为 "ls -la"
#[test]
fn parse_glm_style_bash_command() {
    let response = "bash/command>ls -la";
    let calls = parse_glm_style_tool_calls(response);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "shell");
    assert_eq!(calls[0].1["command"], "ls -la");
}

/// 测试解析 HTTP 请求工具调用
///
/// 验证 GLM 风格的 `http_request/url>URL` 格式能够正确解析，
/// 自动设置 HTTP 方法为 GET。
///
/// # 测试场景
/// - 输入格式：`http_request/url>https://api.example.com/data`
/// - 预期行为：生成 http_request 工具调用，包含 url 和默认 method 参数
#[test]
fn parse_glm_style_http_request() {
    let response = "http_request/url>https://api.example.com/data";
    let calls = parse_glm_style_tool_calls(response);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "http_request");
    assert_eq!(calls[0].1["url"], "https://api.example.com/data");
    assert_eq!(calls[0].1["method"], "GET");
}

/// 测试解析纯 URL（自动转换为 curl）
///
/// 验证当输入仅为 URL 时，能够自动识别并转换为 curl 命令的命令工具调用。
///
/// # 测试场景
/// - 输入格式：`https://example.com/api`（纯 URL）
/// - 预期行为：生成内部 canonical 为 `shell` 的工具调用，命令中包含 curl 和目标 URL
#[test]
fn parse_glm_style_plain_url() {
    let response = "https://example.com/api";
    let calls = parse_glm_style_tool_calls(response);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "shell");
    assert!(calls[0].1["command"].as_str().unwrap().contains("curl"));
}

/// 测试解析 JSON 格式的工具参数
///
/// 验证 GLM 风格支持使用 JSON 对象作为参数，能够正确解析 JSON 结构。
///
/// # 测试场景
/// - 输入格式：`bash/{"command": "echo hello"}`
/// - 预期行为：解析 JSON 参数，正确提取 command 字段
#[test]
fn parse_glm_style_json_args() {
    let response = r#"bash/{"command": "echo hello"}"#;
    let calls = parse_glm_style_tool_calls(response);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "shell");
    assert_eq!(calls[0].1["command"], "echo hello");
}

/// 测试解析多个工具调用
///
/// 验证能够正确解析多行文本中的多个 GLM 风格工具调用。
///
/// # 测试场景
/// - 输入格式：两行，分别包含 bash 和 browser_open 工具调用
/// - 预期行为：返回 2 个工具调用
#[test]
fn parse_glm_style_multiple_calls() {
    let response = r#"bash/command>ls
browser_open/url>https://example.com"#;
    let calls = parse_glm_style_tool_calls(response);
    assert_eq!(calls.len(), 2);
}

/// 测试 GLM 风格在统一解析接口中的集成
///
/// 验证 `parse_tool_calls` 统一接口能够正确处理混合文本，
/// 提取 GLM 风格的工具调用，同时保留前后文本内容。
///
/// # 测试场景
/// - 输入包含：前置文本 + GLM 工具调用 + 后置文本
/// - 预期行为：正确解析工具调用，返回清理后的文本和工具调用列表
#[test]
fn parse_glm_style_tool_call_integration() {
    let response = "Checking...\nbrowser_open/url>https://example.com\nDone";
    let (text, calls) = parse_tool_calls(response);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "shell");
    assert!(text.contains("Checking"));
    assert!(text.contains("Done"));
}

/// 测试拒绝非 HTTP URL 参数（安全性验证）
///
/// 验证安全检查机制：拒绝 javascript: 等非 HTTP/HTTPS 协议的 URL，
/// 防止 XSS（跨站脚本攻击）等安全风险。
///
/// # 测试场景
/// - 输入包含：javascript: 协议的 URL
/// - 预期行为：返回空列表，拒绝执行
#[test]
fn parse_glm_style_rejects_non_http_url_param() {
    let response = "browser_open/url>javascript:alert(1)";
    let calls = parse_glm_style_tool_calls(response);
    assert!(calls.is_empty());
}

// ── 跨别名与 GLM 简化主体测试 ───────────────────────────────────────

/// 测试跨别名闭合标签与 JSON 主体
///
/// 验证当使用 <ACTIONS> 开始标签但用 </invoke> 闭合标签时，
/// 内部的 JSON 格式工具调用仍能正确解析。
///
/// # 测试场景
/// - 输入格式：`<ACTIONS>{"name": "bash", "arguments": {"command": "ls"}}</invoke>`
/// - 预期行为：正确解析 JSON 工具调用，返回空文本
#[test]
fn parse_tool_calls_cross_alias_close_tag_with_json() {
    let input = r#"<tool_call>{"name": "bash", "arguments": {"command": "ls"}}</invoke>"#;
    let (text, calls) = parse_tool_calls(input);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "shell");
    assert_eq!(calls[0].arguments["command"], "ls");
    assert!(text.is_empty());
}

/// 测试跨别名闭合标签中的 GLM 简化格式
///
/// 验证在跨别名标签组合中，GLM 简化格式 `bash>command` 能被正确解析。
///
/// # 测试场景
/// - 输入格式：`<ACTIONS>bash>uname -a</invoke>`
/// - 预期行为：解析为内部 canonical 为 `shell` 的工具调用，command 参数为 "uname -a"
#[test]
fn parse_tool_calls_cross_alias_close_tag_with_glm_shortened() {
    let input = "<tool_call>Bash>uname -a</invoke>";
    let (text, calls) = parse_tool_calls(input);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "shell");
    assert_eq!(calls[0].arguments["command"], "uname -a");
    assert!(text.is_empty());
}

/// 测试匹配标签中的 GLM 简化主体
///
/// 验证在标准的 <ACTIONS>...</ACTIONS> 标签对中，
/// GLM 简化格式能够被正确解析。
///
/// # 测试场景
/// - 输入格式：`<ACTIONS>bash>pwd</ACTIONS>`
/// - 预期行为：解析为内部 canonical 为 `shell` 的工具调用，command 参数为 "pwd"
#[test]
fn parse_tool_calls_glm_shortened_body_in_matched_tags() {
    let input = "<tool_call>Bash>pwdCTIONS";
    let (text, calls) = parse_tool_calls(input);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "shell");
    assert_eq!(calls[0].arguments["command"], "pwd");
    assert!(text.is_empty());
}

/// 测试标签中的 GLM YAML 风格参数
///
/// 验证 YAML 格式的多行参数能够被正确解析，
/// 支持多个参数字段的提取。
///
/// # 测试场景
/// - 输入格式：YAML 风格的多行参数（command + approved）
/// - 预期行为：正确解析多个参数字段
#[test]
fn parse_tool_calls_glm_yaml_style_in_tags() {
    let input = "<tool_call>Bash>\ncommand: date\napproved: true</invoke>";
    let (text, calls) = parse_tool_calls(input);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "shell");
    assert_eq!(calls[0].arguments["command"], "date");
    assert_eq!(calls[0].arguments["approved"], true);
    assert!(text.is_empty());
}

/// 测试标签中的属性风格参数
///
/// 验证 XML 属性风格的参数定义能够被正确解析，
/// 类似于 `<bash command="date" />` 的自闭合格式。
///
/// # 测试场景
/// - 输入格式：属性风格的单行参数定义
/// - 预期行为：正确提取属性值作为工具参数
#[test]
fn parse_tool_calls_attribute_style_in_tags() {
    let input = r#"<tool_call>Bash command="date" />CTIONS"#;
    let (text, calls) = parse_tool_calls(input);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "shell");
    assert_eq!(calls[0].arguments["command"], "date");
    assert!(text.is_empty());
}

/// 测试跨别名标签中的文件读取工具（简化格式）
///
/// 验证 file_read 工具的简化格式在跨别名标签中能够正确解析，
/// 确保路径参数被正确提取。
///
/// # 测试场景
/// - 输入格式：`<ACTIONS>file_read path=".env" /></invoke>`
/// - 预期行为：解析为 file_read 工具，path 参数为 ".env"
#[test]
fn parse_tool_calls_file_read_shortened_in_cross_alias() {
    let input = r#"<tool_call>file_read path=".env" /></invoke>"#;
    let (text, calls) = parse_tool_calls(input);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "file_read");
    assert_eq!(calls[0].arguments["path"], ".env");
    assert!(text.is_empty());
}

/// 测试未闭合的 GLM 简化格式（无闭合标签）
///
/// 验证即使缺少闭合标签，GLM 简化格式仍能被容错解析，
/// 这对于处理流式输出或不完整响应非常重要。
///
/// # 测试场景
/// - 输入格式：`<ACTIONS>bash>ls -la`（无闭合标签）
/// - 预期行为：仍能正确解析工具调用
#[test]
fn parse_tool_calls_unclosed_glm_shortened_no_close_tag() {
    let input = "<tool_call>Bash>ls -la";
    let (text, calls) = parse_tool_calls(input);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "shell");
    assert_eq!(calls[0].arguments["command"], "ls -la");
    assert!(text.is_empty());
}

/// 测试跨别名工具调用前后的文本保留
///
/// 验证在跨别名格式的工具调用前后，
/// 普通文本内容能够被正确保留和返回。
///
/// # 测试场景
/// - 输入包含：前置说明文本 + 工具调用 + 后置说明文本
/// - 预期行为：工具调用被提取，前后文本被保留在返回的文本中
#[test]
fn parse_tool_calls_text_before_cross_alias() {
    let input = "Let me check that.\n<tool_call>bash>uname -a</invoke>\nDone.";
    let (text, calls) = parse_tool_calls(input);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "shell");
    assert_eq!(calls[0].arguments["command"], "uname -a");
    assert!(text.contains("Let me check that."));
    assert!(text.contains("Done."));
}

/// 测试 GLM 简化主体中的 URL 转换为 curl 命令
///
/// 验证当 bash 工具接收 URL 作为值时，
/// 能够自动转换为包含 curl 的完整命令。
///
/// # 测试场景
/// - 输入格式：`bash>https://example.com/api`
/// - 预期行为：生成包含 curl 和 URL 的命令字符串
#[test]
fn parse_glm_shortened_body_url_to_curl() {
    let call = parse_glm_shortened_body("bash>https://example.com/api").unwrap();
    assert_eq!(call.name, "shell");
    let cmd = call.arguments["command"].as_str().unwrap();
    assert!(cmd.contains("curl"));
    assert!(cmd.contains("example.com"));
}

/// 测试 browser_open 别名映射到命令工具命令
///
/// 验证 browser_open 工具作为命令工具别名，
/// 在简化格式中仍能生成规范的内部 `shell` 工具调用，
/// 并正确设置 "command" 参数。
///
/// # 测试场景
/// - 输入格式：`browser_open>https://example.com`
/// - 预期行为：生成内部 canonical 为 `shell` 的工具调用，命令包含 curl
#[test]
fn parse_glm_shortened_body_browser_open_maps_to_shell_command() {
    let call = parse_glm_shortened_body("browser_open>https://example.com").unwrap();
    assert_eq!(call.name, "shell");
    let cmd = call.arguments["command"].as_str().unwrap();
    assert!(cmd.contains("curl"));
    assert!(cmd.contains("example.com"));
}

/// 测试 memory_recall 工具的默认参数
///
/// 验证 memory_recall 工具使用 "query" 作为默认参数名，
/// 简化格式中的值能够正确映射到该参数。
///
/// # 测试场景
/// - 输入格式：`memory_recall>recent meetings`
/// - 预期行为：query 参数被设置为 "recent meetings"
#[test]
fn parse_glm_shortened_body_memory_recall() {
    let call = parse_glm_shortened_body("memory_recall>recent meetings").unwrap();
    assert_eq!(call.name, "memory_recall");
    assert_eq!(call.arguments["query"], "recent meetings");
}

/// 测试函数风格别名的映射
///
/// 验证 `sendmessage` 函数风格调用能够被映射到
/// 规范的 `message_send` 工具名，并正确提取参数。
///
/// # 测试场景
/// - 输入格式：`sendmessage(channel="alerts", message="hi")`
/// - 预期行为：映射到 message_send 工具，参数被正确提取
#[test]
fn parse_glm_shortened_body_function_style_alias_maps_to_message_send() {
    let call = parse_glm_shortened_body(r#"sendmessage(channel="alerts", message="hi")"#).unwrap();
    assert_eq!(call.name, "message_send");
    assert_eq!(call.arguments["channel"], "alerts");
    assert_eq!(call.arguments["message"], "hi");
}

/// 测试拒绝空输入
///
/// 验证解析器对空字符串和纯空白字符串的处理，
/// 应返回 None 表示无效输入。
///
/// # 测试场景
/// - 输入：空字符串 "" 和纯空白 "   "
/// - 预期行为：均返回 None
#[test]
fn parse_glm_shortened_body_rejects_empty() {
    assert!(parse_glm_shortened_body("").is_none());
    assert!(parse_glm_shortened_body("   ").is_none());
}

/// 测试拒绝无效工具名
///
/// 验证工具名称的合法性检查，
/// 包含特殊字符（如连字符、空格）的工具名应被拒绝，
/// 防止注入攻击或解析错误。
///
/// # 测试场景
/// - 输入包含：带连字符的工具名、带空格的工具名
/// - 预期行为：均返回 None，拒绝解析
#[test]
fn parse_glm_shortened_body_rejects_invalid_tool_name() {
    assert!(parse_glm_shortened_body("not-a-tool>value").is_none());
    assert!(parse_glm_shortened_body("tool name>value").is_none());
}
