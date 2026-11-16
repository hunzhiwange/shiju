//! 工具调用解析功能测试模块
//!
//! 本模块包含 `parse_tool_calls` 函数的全面测试用例，验证代理运行时
//! 能够正确解析来自不同 AI 提供商的各种格式的工具调用响应。
//!
//! # 支持的格式
//!
//! 测试覆盖以下工具调用格式：
//! - **XML 标签格式**：`<tool_call name="...">...</tool_call>`
//! - **XML 嵌套格式**：`<tool_call><name><args/></name></tool_call>`
//! - **JSON 格式**：在标签内直接嵌入 JSON 对象
//! - **OpenAI 格式**：包含 `tool_calls` 数组的标准 OpenAI 响应结构
//! - **Markdown 代码块格式**：` ```tool_call ` 或 ` ```invoke ` 围栏
//! - **内联属性格式**：类函数调用的属性传递风格
//! - **MiniMax 格式**：MiniMax 提供商特有的 XML 参数格式
//!
//! # 安全考虑
//!
//! 测试确保只有带有明确包装器的 JSON 才会被解析为工具调用，
//! 防止提示注入攻击（恶意内容伪装成工具调用 JSON）。

use crate::parsing::parse_tool_calls;
use crate::parsing::tool_call_formats::parse_perl_style_tool_calls;

/// 测试从响应中提取单个工具调用
///
/// 验证解析器能够从包含文本和单个工具调用的混合内容中
/// 正确提取工具名称、参数，并分离出纯文本部分。
#[test]
fn parse_tool_calls_extracts_single_call() {
    let response = r#"Let me check that.
<tool_call>
{"name": "bash", "arguments": {"command": "ls -la"}}
</tool_call>"#;

    let (text, calls) = parse_tool_calls(response);
    assert_eq!(text, "Let me check that.");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "shell");
    assert_eq!(calls[0].arguments.get("command").unwrap().as_str().unwrap(), "ls -la");
}

/// 测试从响应中提取多个工具调用
///
/// 验证解析器能够处理连续的多个工具调用标签，
/// 并正确提取每个调用的名称和参数。
#[test]
fn parse_tool_calls_extracts_multiple_calls() {
    let response = r#"<tool_call>
{"name": "file_read", "arguments": {"path": "a.txt"}}
</tool_call>
<tool_call>
{"name": "file_read", "arguments": {"path": "b.txt"}}
</tool_call>"#;

    let (_, calls) = parse_tool_calls(response);
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].name, "file_read");
    assert_eq!(calls[1].name, "file_read");
}

/// 测试无工具调用时返回纯文本
///
/// 当响应中不包含任何工具调用标签时，应原样返回文本，
/// 且工具调用列表应为空。
#[test]
fn parse_tool_calls_returns_text_only_when_no_calls() {
    let response = "Just a normal response with no tools.";
    let (text, calls) = parse_tool_calls(response);
    assert_eq!(text, "Just a normal response with no tools.");
    assert!(calls.is_empty());
}

/// 测试处理格式错误的 JSON
///
/// 当工具调用标签内的 JSON 无效时，解析器应优雅地跳过该调用，
/// 同时保留标签外的文本内容。
#[test]
fn parse_tool_calls_handles_malformed_json() {
    let response = r#"<tool_call>
not valid json
</tool_call>
Some text after."#;

    let (text, calls) = parse_tool_calls(response);
    assert!(calls.is_empty());
    assert!(text.contains("Some text after."));
}

/// 测试工具调用前后都有文本的情况
///
/// 验证解析器能够正确保留工具调用标签前后的文本内容，
/// 实现文本与工具调用的完整分离。
#[test]
fn parse_tool_calls_text_before_and_after() {
    let response = r#"Before text.
<tool_call>
{"name": "bash", "arguments": {"command": "echo hi"}}
</tool_call>
After text."#;

    let (text, calls) = parse_tool_calls(response);
    assert!(text.contains("Before text."));
    assert!(text.contains("After text."));
    assert_eq!(calls.len(), 1);
}

/// 测试解析 OpenAI 标准格式的工具调用
///
/// OpenAI 格式的响应将工具调用放在 `tool_calls` 数组中，
/// 每个调用包含 `type`、`function.name` 和 `function.arguments` 字段。
/// 参数是 JSON 字符串而非对象，需要二次解析。
#[test]
fn parse_tool_calls_handles_openai_format() {
    let response = r#"{"content": "Let me check that for you.", "tool_calls": [{"type": "function", "function": {"name": "bash", "arguments": "{\"command\": \"ls -la\"}"}}]}"#;

    let (text, calls) = parse_tool_calls(response);
    assert_eq!(text, "Let me check that for you.");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "shell");
    assert_eq!(calls[0].arguments.get("command").unwrap().as_str().unwrap(), "ls -la");
}

/// 测试解析 OpenAI 格式的多个工具调用
///
/// 验证解析器能够正确处理包含多个工具调用的 OpenAI 响应格式。
#[test]
fn parse_tool_calls_handles_openai_format_multiple_calls() {
    let response = r#"{"tool_calls": [{"type": "function", "function": {"name": "file_read", "arguments": "{\"path\": \"a.txt\"}"}}, {"type": "function", "function": {"name": "file_read", "arguments": "{\"path\": \"b.txt\"}"}}]}"#;

    let (_, calls) = parse_tool_calls(response);
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].name, "file_read");
    assert_eq!(calls[1].name, "file_read");
}

/// 测试 OpenAI 格式中无 content 字段的情况
///
/// 某些 AI 提供商在返回工具调用时可能不包含 `content` 字段，
/// 此时文本部分应为空，但仍需正确解析工具调用。
#[test]
fn parse_tool_calls_openai_format_without_content() {
    let response = r#"{"tool_calls": [{"type": "function", "function": {"name": "memory_recall", "arguments": "{}"}}]}"#;

    let (text, calls) = parse_tool_calls(response);
    assert!(text.is_empty());
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "memory_recall");
}

/// 测试解析带 message 包装的 OpenAI 响应
///
/// 某些 OpenAI 兼容的 API 可能将响应包装在 `message` 对象中，
/// 同时包含 `role`、`content` 和 `tool_calls` 字段。
/// 本测试验证解析器能够处理这种嵌套结构。
#[test]
fn parse_tool_calls_handles_openai_message_wrapper_with_content() {
    let response = r#"{
        "message": {
            "role": "assistant",
            "content": "OK<think>plan</think>\nI will call a tool.",
            "tool_calls": [
                {
                    "id": "chatcmpl-tool-a18c01b8849eb05d",
                    "type": "function",
                    "function": {
                                "name": "bash",
                        "arguments": "{\"command\": \"ls -la\"}"
                    }
                }
            ]
        },
        "finish_reason": "tool_calls"
    }"#;

    let (text, calls) = parse_tool_calls(response);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "shell");
    assert_eq!(calls[0].arguments.get("command").unwrap().as_str().unwrap(), "ls -la");
    assert!(text.contains("I will call a tool."));
}

/// 测试解析完整的 OpenAI choices/message 嵌套结构
///
/// 标准 OpenAI API 响应格式包含 `choices` 数组，每个元素包含
/// `message` 对象。本测试验证解析器能够处理这种完整的响应结构，
/// 并正确提取 `tool_call_id` 字段。
#[test]
fn parse_tool_calls_handles_openai_choices_message_wrapper() {
    let response = r#"{
        "id": "chatcmpl-123",
        "choices": [
            {
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Checking now.",
                    "tool_calls": [
                        {
                            "id": "call_1",
                            "type": "function",
                            "function": {
                                "name": "bash",
                                "arguments": "{\"command\":\"pwd\"}"
                            }
                        }
                    ]
                },
                "finish_reason": "tool_calls"
            }
        ]
    }"#;

    let (text, calls) = parse_tool_calls(response);
    assert_eq!(text, "Checking now.");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "shell");
    assert_eq!(calls[0].arguments.get("command").unwrap().as_str().unwrap(), "pwd");
    assert_eq!(calls[0].tool_call_id.as_deref(), Some("call_1"));
}

/// 测试保留 OpenAI 格式中的 tool_call_id
///
/// OpenAI 为每个工具调用分配唯一的 ID，后续提交结果时需要此 ID。
/// 本测试验证解析器能够正确提取并保留该字段。
#[test]
fn parse_tool_calls_preserves_openai_tool_call_ids() {
    let response = r#"{"tool_calls":[{"id":"call_42","function":{"name":"bash","arguments":"{\"command\":\"pwd\"}"}}]}"#;
    let (_, calls) = parse_tool_calls(response);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].tool_call_id.as_deref(), Some("call_42"));
}

/// 测试工具调用标签内包含 Markdown JSON 代码块
///
/// 某些模型可能在 `<tool_call>` 标签内再包裹一层 Markdown 代码块，
/// 解析器需要能够剥离这层包装，提取出真正的 JSON 内容。
#[test]
fn parse_tool_calls_handles_markdown_json_inside_tool_call_tag() {
    let response = r#"<tool_call>
```json
{"name": "file_write", "arguments": {"path": "test.py", "content": "print('ok')"}}
```
</tool_call>"#;

    let (text, calls) = parse_tool_calls(response);
    assert!(text.is_empty());
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "file_write");
    assert_eq!(calls[0].arguments.get("path").unwrap().as_str().unwrap(), "test.py");
}

/// 测试工具调用标签内包含噪音文本的情况
///
/// 模型可能在 JSON 前添加解释性文本，解析器应能够
/// 忽略这些噪音，从标签体中提取出有效的 JSON 对象。
#[test]
fn parse_tool_calls_handles_noisy_tool_call_tag_body() {
    let response = r#"<tool_call>
I will now call the tool with this payload:
{"name": "bash", "arguments": {"command": "pwd"}}
</tool_call>"#;

    let (text, calls) = parse_tool_calls(response);
    assert!(text.is_empty());
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "shell");
    assert_eq!(calls[0].arguments.get("command").unwrap().as_str().unwrap(), "pwd");
}

/// 测试解析内联属性格式的工具调用（使用 send_message 别名）
///
/// 某些模型可能使用类 XML 属性的语法而非 JSON 对象来传递参数。
/// 本测试验证 `send_message` 别名能够正确映射到 `message_send` 工具名，
/// 且属性值被正确解析为参数。
#[test]
fn parse_tool_calls_handles_tool_call_inline_attributes_with_send_message_alias() {
    let response = r#"<tool_call>send_message channel="user_channel" message="Hello! How can I assist you today?"></tool_call>"#;

    let (text, calls) = parse_tool_calls(response);
    assert!(text.is_empty());
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "message_send");
    assert_eq!(calls[0].arguments.get("channel").unwrap().as_str().unwrap(), "user_channel");
    assert_eq!(
        calls[0].arguments.get("message").unwrap().as_str().unwrap(),
        "Hello! How can I assist you today?"
    );
}

/// 测试解析类函数调用语法的工具调用
///
/// 验证解析器能够处理 `name(arg1="value1", arg2="value2")` 这种
/// 类编程语言函数调用的参数传递格式。
#[test]
fn parse_tool_calls_handles_tool_call_function_style_arguments() {
    let response = r#"<tool_call> message_send(channel="general", message="test")></tool_call>"#;

    let (text, calls) = parse_tool_calls(response);
    assert!(text.is_empty());
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "message_send");
    assert_eq!(calls[0].arguments.get("channel").unwrap().as_str().unwrap(), "general");
    assert_eq!(calls[0].arguments.get("message").unwrap().as_str().unwrap(), "test");
}

/// 测试解析 XML 嵌套格式的工具载荷
///
/// 某些模型可能将工具调用表示为嵌套的 XML 元素，
/// 工具名作为根元素名，参数作为子元素。
/// 本测试验证解析器能够从这种结构中提取工具名和参数。
#[test]
fn parse_tool_calls_handles_xml_nested_tool_payload() {
    let response = r#"<tool_call>
<memory_recall>
<query>project roadmap</query>
</memory_recall>
</tool_call>"#;

    let (text, calls) = parse_tool_calls(response);
    assert!(text.is_empty());
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "memory_recall");
    assert_eq!(calls[0].arguments.get("query").unwrap().as_str().unwrap(), "project roadmap");
}

/// 测试忽略 XML 格式中的 thinking 包装标签
///
/// 模型可能在工具调用前输出 `<thinking>` 标签来解释其推理过程，
/// 解析器应忽略这些思考标签，只提取真正的工具调用内容。
#[test]
fn parse_tool_calls_ignores_xml_thinking_wrapper() {
    let response = r#"<tool_call>
<thinking>Need to inspect memory first</thinking>
<memory_recall>
<query>recent deploy notes</query>
</memory_recall>
</tool_call>"#;

    let (text, calls) = parse_tool_calls(response);
    assert!(text.is_empty());
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "memory_recall");
    assert_eq!(calls[0].arguments.get("query").unwrap().as_str().unwrap(), "recent deploy notes");
}

/// 测试解析 XML 标签内嵌 JSON 参数的混合格式
///
/// 工具名通过 XML 标签名指定，参数通过 JSON 对象传递。
/// 这种混合格式结合了 XML 的可读性和 JSON 的结构化优势。
#[test]
fn parse_tool_calls_handles_xml_with_json_arguments() {
    let response = r#"<tool_call>
<bash>{"command":"pwd"}</bash>
</tool_call>"#;

    let (text, calls) = parse_tool_calls(response);
    assert!(text.is_empty());
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "shell");
    assert_eq!(calls[0].arguments.get("command").unwrap().as_str().unwrap(), "pwd");
}

/// 测试解析 Markdown tool_call 代码块格式
///
/// 某些模型使用 Markdown 代码块语法来表示工具调用，
/// 语法为 ` ```tool_call ` 后跟 JSON 内容。解析器应提取工具调用
/// 并从文本中移除代码块标记，同时保留前后的文本内容。
#[test]
fn parse_tool_calls_handles_markdown_tool_call_fence() {
    let response = r#"I'll check that.
```tool_call
{"name": "bash", "arguments": {"command": "pwd"}}
```
Done."#;

    let (text, calls) = parse_tool_calls(response);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "shell");
    assert_eq!(calls[0].arguments.get("command").unwrap().as_str().unwrap(), "pwd");
    assert!(text.contains("I'll check that."));
    assert!(text.contains("Done."));
    assert!(!text.contains("```tool_call"));
}

/// 测试解析 Markdown tool-call 代码块与 `</tool_call>` 标签的混合关闭
///
/// 验证解析器能够处理 Markdown 代码块开始但使用 XML 标签关闭的
/// 混合格式，同时正确提取工具调用并保留周围文本。
#[test]
fn parse_tool_calls_handles_markdown_tool_call_hybrid_close_tag() {
    let response = r#"Preface
```tool-call
{"name": "bash", "arguments": {"command": "date"}}
</tool_call>
Tail"#;

    let (text, calls) = parse_tool_calls(response);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "shell");
    assert_eq!(calls[0].arguments.get("command").unwrap().as_str().unwrap(), "date");
    assert!(text.contains("Preface"));
    assert!(text.contains("Tail"));
    assert!(!text.contains("```tool-call"));
}

/// 测试解析 Markdown invoke 代码块格式
///
/// `invoke` 是 `tool_call` 的另一个别名，功能相同。
/// 解析器应将 ` ```invoke ` 代码块中的 JSON 解析为工具调用。
#[test]
fn parse_tool_calls_handles_markdown_invoke_fence() {
    let response = r#"Checking.
```invoke
{"name": "bash", "arguments": {"command": "date"}}
```
Done."#;

    let (text, calls) = parse_tool_calls(response);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "shell");
    assert_eq!(calls[0].arguments.get("command").unwrap().as_str().unwrap(), "date");
    assert!(text.contains("Checking."));
    assert!(text.contains("Done."));
}

/// 测试解析 `tool <name>` Markdown 代码块格式
///
/// xAI 的 Grok 模型使用 ` ```tool <工具名> ` 格式，将工具名放在
/// 代码块语言标识符中。解析器应从代码块语言标识符提取工具名，
/// 并从代码块内容解析参数。关联 Issue #1420。
#[test]
fn parse_tool_calls_handles_tool_name_fence_format() {
    let response = r#"I'll write a test file.
```tool file_write
{"path": "/home/user/test.txt", "content": "Hello world"}
```
Done."#;

    let (text, calls) = parse_tool_calls(response);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "file_write");
    assert_eq!(calls[0].arguments.get("path").unwrap().as_str().unwrap(), "/home/user/test.txt");
    assert!(text.contains("I'll write a test file."));
    assert!(text.contains("Done."));
}

/// 测试解析 `tool bash` Markdown 代码块格式的命令工具调用
///
/// 验证 ` ```tool bash ` 格式能够正确解析，并收敛到内部 `shell` 工具调用。
/// 关联 Issue #1420。
#[test]
fn parse_tool_calls_handles_tool_name_fence_bash() {
    let response = r#"```tool bash
{"command": "ls -la"}
```"#;

    let (_text, calls) = parse_tool_calls(response);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "shell");
    assert_eq!(calls[0].arguments.get("command").unwrap().as_str().unwrap(), "ls -la");
}

/// 测试解析多个 `tool <name>` Markdown 代码块
///
/// 验证解析器能够处理响应中包含多个 ` ```tool <name> ` 代码块的情况，
/// 正确提取每个工具调用并保留代码块之间的文本。
#[test]
fn parse_tool_calls_handles_multiple_tool_name_fences() {
    let response = r#"First, I'll write a file.
```tool file_write
{"path": "/tmp/a.txt", "content": "A"}
```
Then read it.
```tool file_read
{"path": "/tmp/a.txt"}
```
Done."#;

    let (text, calls) = parse_tool_calls(response);
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].name, "file_write");
    assert_eq!(calls[1].name, "file_read");
    assert!(text.contains("First, I'll write a file."));
    assert!(text.contains("Then read it."));
    assert!(text.contains("Done."));
}

/// 测试解析 `<toolcall>` 标签别名
///
/// `toolcall` 是 `tool_call` 标签的一个替代拼写，解析器应同等处理。
#[test]
fn parse_tool_calls_handles_toolcall_tag_alias() {
    let response = r#"<toolcall>
{"name": "bash", "arguments": {"command": "date"}}
</toolcall>"#;

    let (text, calls) = parse_tool_calls(response);
    assert!(text.is_empty());
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "shell");
    assert_eq!(calls[0].arguments.get("command").unwrap().as_str().unwrap(), "date");
}

/// 测试解析 `<tool-call>` 标签别名
///
/// `tool-call` 是使用连字符的标签格式变体，解析器应同等处理。
#[test]
fn parse_tool_calls_handles_tool_dash_call_tag_alias() {
    let response = r#"<tool-call>
{"name": "bash", "arguments": {"command": "whoami"}}
</tool-call>"#;

    let (text, calls) = parse_tool_calls(response);
    assert!(text.is_empty());
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "shell");
    assert_eq!(calls[0].arguments.get("command").unwrap().as_str().unwrap(), "whoami");
}

/// 测试解析 `<invoke>` 标签别名
///
/// `invoke` 是另一个工具调用标签别名，语义与 `tool_call` 相同。
#[test]
fn parse_tool_calls_handles_invoke_tag_alias() {
    let response = r#"<invoke>
{"name": "bash", "arguments": {"command": "uptime"}}
</invoke>"#;

    let (text, calls) = parse_tool_calls(response);
    assert!(text.is_empty());
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "shell");
    assert_eq!(calls[0].arguments.get("command").unwrap().as_str().unwrap(), "uptime");
}

/// 测试解析 MiniMax 提供商特有的工具调用格式
///
/// MiniMax 使用 `<minimax:tool_call>` 包装器，内部使用 `<invoke>` 标签，
/// 参数通过 `<parameter name="...">` 元素传递。这种格式完全基于 XML，
/// 不使用 JSON。
#[test]
fn parse_tool_calls_handles_minimax_invoke_parameter_format() {
    let response = r#"<minimax:tool_call>
<invoke name="shell">
<parameter name="command">sqlite3 /tmp/test.db ".tables"</parameter>
</invoke>
</minimax:tool_call>"#;

    let (text, calls) = parse_tool_calls(response);
    assert!(text.is_empty());
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "shell");
    assert_eq!(
        calls[0].arguments.get("command").unwrap().as_str().unwrap(),
        r#"sqlite3 /tmp/test.db ".tables""#
    );
}

/// 测试 MiniMax 格式包含周围文本的情况
///
/// 验证 MiniMax 格式的工具调用能够与前后文本正确分离，
/// 并正确解析包含特殊字符（如单引号）的属性值。
#[test]
fn parse_tool_calls_handles_minimax_invoke_with_surrounding_text() {
    let response = r#"Preface
<minimax:tool_call>
<invoke name='http_request'>
<parameter name='url'>https://example.com</parameter>
<parameter name='method'>GET</parameter>
</invoke>
</minimax:tool_call>
Tail"#;

    let (text, calls) = parse_tool_calls(response);
    assert!(text.contains("Preface"));
    assert!(text.contains("Tail"));
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "http_request");
    assert_eq!(calls[0].arguments.get("url").unwrap().as_str().unwrap(), "https://example.com");
    assert_eq!(calls[0].arguments.get("method").unwrap().as_str().unwrap(), "GET");
}

/// 测试 MiniMax 别名与标准标签的交叉关闭
///
/// 验证解析器能够处理使用 `<tool_call>` 开启但使用
/// `</minimax:toolcall>` 关闭的交叉标签格式，确保容错性。
#[test]
fn parse_tool_calls_handles_minimax_toolcall_alias_and_cross_close_tag() {
    let response = r#"<tool_call>
{"name":"bash","arguments":{"command":"date"}}
</minimax:toolcall>"#;

    let (text, calls) = parse_tool_calls(response);
    assert!(text.is_empty());
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "shell");
    assert_eq!(calls[0].arguments.get("command").unwrap().as_str().unwrap(), "date");
}

/// 测试解析 Perl 风格的工具调用块
///
/// 某些模型可能输出类似 Perl 语法的工具调用格式，
/// 使用 `=>` 箭头和 `--` 前缀的参数。
/// 函数专门处理这种格式。
#[test]
fn parse_tool_calls_handles_perl_style_tool_call_blocks() {
    let response = r#"TOOL_CALL
{tool => "bash", args => { --command "uname -a" }}}
/TOOL_CALL"#;

    let calls = parse_perl_style_tool_calls(response);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "shell");
    assert_eq!(calls[0].arguments.get("command").unwrap().as_str().unwrap(), "uname -a");
}

/// 测试恢复未关闭的工具调用标签（JSON 格式）
///
/// 当模型输出的工具调用标签缺少关闭标签时，解析器应尝试
/// 恢复并提取其中的 JSON 工具调用，而不是直接丢弃。
#[test]
fn parse_tool_calls_recovers_unclosed_tool_call_with_json() {
    let response = r#"I will call the tool now.
<tool_call>
{"name": "bash", "arguments": {"command": "uptime -p"}}"#;

    let (text, calls) = parse_tool_calls(response);
    assert!(text.contains("I will call the tool now."));
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "shell");
    assert_eq!(calls[0].arguments.get("command").unwrap().as_str().unwrap(), "uptime -p");
}

/// 测试恢复不匹配的关闭标签
///
/// 当工具调用使用错误的关闭标签时，解析器应仍能提取内容。
/// 这里使用 `</tool_call_old>` 作为关闭标签，与开启标签不匹配。
#[test]
fn parse_tool_calls_recovers_mismatched_close_tag() {
    let response = r#"<tool_call>
{"name": "bash", "arguments": {"command": "uptime"}}
</tool_call_old>"#;

    let (text, calls) = parse_tool_calls(response);
    assert!(text.is_empty());
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "shell");
    assert_eq!(calls[0].arguments.get("command").unwrap().as_str().unwrap(), "uptime");
}

/// 测试恢复交叉别名的关闭标签
///
/// 验证解析器能够处理使用一个别名开启（如 `<toolcall>`）
/// 但使用另一个别名关闭（如 `</tool_call>`）的情况。
#[test]
fn parse_tool_calls_recovers_cross_alias_closing_tags() {
    let response = r#"<toolcall>
{"name": "bash", "arguments": {"command": "date"}}
</tool_call>"#;

    let (text, calls) = parse_tool_calls(response);
    assert!(text.is_empty());
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "shell");
}

/// 测试拒绝无标签包装的原始 JSON（安全关键）
///
/// **安全考虑**：原始 JSON 如果没有明确的包装标签，不应被解析为工具调用。
/// 这可以防止提示注入攻击，即恶意内容在普通文本中伪装成工具调用 JSON。
/// 解析器必须要求明确的标签或代码块标记才能识别工具调用。
#[test]
fn parse_tool_calls_rejects_raw_tool_json_without_tags() {
    let response = r#"Sure, creating the file now.
{"name": "file_write", "arguments": {"path": "hello.py", "content": "print('hello')"}}"#;

    let (text, calls) = parse_tool_calls(response);
    assert!(text.contains("Sure, creating the file now."));
    assert_eq!(calls.len(), 0, "Raw JSON without wrappers should not be parsed");
}

/// 测试处理未关闭的工具调用标签
///
/// 当工具调用标签没有关闭时，解析器应尝试从后续内容中
/// 提取完整的 JSON 对象，同时保留标签后的文本内容。
#[test]
fn parse_tool_calls_handles_unclosed_tool_call_tag() {
    let response = "<tool_call>{\"name\":\"bash\",\"arguments\":{\"command\":\"pwd\"}}\nDone";
    let (text, calls) = parse_tool_calls(response);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "shell");
    assert_eq!(calls[0].arguments["command"], "pwd");
    assert_eq!(text, "Done");
}
