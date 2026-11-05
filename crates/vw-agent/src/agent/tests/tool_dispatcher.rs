//! 工具调度器测试模块
//!
//! 本模块提供对 `NativeToolDispatcher` 和 `XmlToolDispatcher` 的全面测试覆盖，
//! 验证工具调度器在以下方面的行为：
//!
//! - 工具规范的发送策略（是否向模型发送工具定义）
//! - 响应解析（从不同格式的模型响应中提取工具调用）
//! - 参数处理（字符串化参数、嵌套 JSON 等）
//! - 结果格式化（将工具执行结果转换为对话消息）
//! - 历史记录转换（将对话历史转换为提供商消息格式）
//! - 提示指令生成（为模型生成工具使用说明）
//!
//! # 测试策略
//!
//! 采用黑盒测试方法，通过 `ToolDispatcher` trait 的公共接口验证行为，
//! 使用 `ScriptedProvider` 模拟模型响应，使用 `EchoTool` 作为测试工具。

use crate::app::agent::agent::dispatcher::{
    NativeToolDispatcher, ToolDispatcher, XmlToolDispatcher,
};
use crate::app::agent::providers::{
    ChatMessage, ChatResponse, ConversationMessage, ToolCall, ToolResultMessage,
};

use super::helpers::{EchoTool, ScriptedProvider, build_tool_execution_result, text_response};

/// 测试 NativeToolDispatcher 是否发送工具规范
///
/// 验证 `NativeToolDispatcher` 的 `should_send_tool_specs()` 方法返回 `true`，
/// 表示该调度器会在请求中将工具定义发送给模型。
///
/// # 测试流程
///
/// 1. 创建带有模拟提供商的 Agent，使用 NativeToolDispatcher
/// 2. 执行一轮对话
/// 3. 验证调度器返回 `true`
#[tokio::test]
async fn native_dispatcher_sends_tool_specs() {
    // 创建脚本化提供商，预设返回简单的文本响应
    let provider = Box::new(ScriptedProvider::new(vec![text_response("ok")]));

    // 构建使用 NativeToolDispatcher 的 Agent
    let mut agent = super::helpers::build_agent_with(
        provider,
        vec![Box::new(EchoTool)],
        Box::new(NativeToolDispatcher),
    );

    // 执行一轮对话
    let _ = agent.turn("hi").await.unwrap();

    // 验证 NativeToolDispatcher 应该发送工具规范
    let dispatcher = NativeToolDispatcher;
    assert!(dispatcher.should_send_tool_specs());
}

/// 测试 XmlToolDispatcher 不发送工具规范
///
/// 验证 `XmlToolDispatcher` 的 `should_send_tool_specs()` 方法返回 `false`，
/// 表示该调度器不在请求中发送工具定义，而是通过提示指令引导模型使用 XML 格式调用工具。
#[test]
fn xml_dispatcher_does_not_send_tool_specs() {
    let dispatcher = XmlToolDispatcher;
    // XmlToolDispatcher 依赖提示指令而非工具规范
    assert!(!dispatcher.should_send_tool_specs());
}

/// 测试 NativeToolDispatcher 处理字符串化的参数
///
/// 验证调度器能正确解析以 JSON 字符串形式提供的工具调用参数。
/// 某些模型提供商可能将参数作为字符串而非对象返回。
///
/// # 测试场景
///
/// - 工具调用参数为 JSON 字符串：`{"message": "hello"}`
/// - 验证解析后的参数可正确访问
#[tokio::test]
async fn native_dispatcher_handles_stringified_arguments() {
    let dispatcher = NativeToolDispatcher;

    // 构造包含字符串化参数的响应
    let response = ChatResponse {
        text: Some(String::new()),
        tool_calls: vec![ToolCall {
            kind: Default::default(),
            namespace: None,
            id: "tc1".into(),
            name: "echo".into(),
            arguments: r#"{"message": "hello"}"#.into(), // 参数为 JSON 字符串
        }],
        usage: None,
        reasoning_content: None,
    };

    // 解析响应并验证结果
    let (_, calls) = dispatcher.parse_response(&response);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "echo");
    // 验证参数被正确解析为可访问的 JSON 对象
    assert_eq!(calls[0].arguments.get("message").unwrap().as_str().unwrap(), "hello");
}

/// 测试 XmlToolDispatcher 处理嵌套 JSON 参数
///
/// 验证调度器能正确解析包含嵌套 JSON 对象的工具调用参数。
/// XML 调度器从文本中提取 XML 标签内的 JSON 内容。
///
/// # 测试场景
///
/// - 工具调用嵌入在 `<tool_call>` 标签中
/// - 参数中包含嵌套的 JSON 对象（如文件内容）
/// - 验证嵌套 JSON 被正确解析
#[test]
fn xml_dispatcher_handles_nested_json() {
    // 构造包含 XML 格式工具调用的响应，参数中包含嵌套 JSON
    let response = ChatResponse {
        text: Some(
            r#"<tool_call>
{"name": "file_write", "arguments": {"path": "test.json", "content": "{\"key\": \"value\"}"}}
</tool_call>"#
                .into(),
        ),
        tool_calls: vec![],
        usage: None,
        reasoning_content: None,
    };

    let dispatcher = XmlToolDispatcher;
    let (_, calls) = dispatcher.parse_response(&response);

    // 验证工具调用被正确提取和解析
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "file_write");
    assert_eq!(calls[0].arguments.get("path").unwrap().as_str().unwrap(), "test.json");
}

/// 测试 XmlToolDispatcher 处理空的工具调用标签
///
/// 验证当 XML 标签为空或仅包含空白时，调度器能优雅处理而不崩溃。
/// 这是容错性测试，确保格式异常不会导致系统错误。
#[test]
fn xml_dispatcher_handles_empty_tool_call_tag() {
    // 构造包含空工具调用标签的响应
    let response = ChatResponse {
        text: Some("<tool_call>\n</tool_call>\nSome text".into()),
        tool_calls: vec![],
        usage: None,
        reasoning_content: None,
    };

    let dispatcher = XmlToolDispatcher;
    let (text, calls) = dispatcher.parse_response(&response);

    // 验证空标签被忽略，其他文本被保留
    assert!(calls.is_empty());
    assert!(text.contains("Some text"));
}

/// 测试 XmlToolDispatcher 处理未闭合的工具调用标签
///
/// 验证当 XML 标签未正确闭合时，调度器不会崩溃，
/// 而是将整个内容视为普通文本处理。
///
/// # 容错行为
///
/// 未闭合的标签不应触发错误，而是安全地跳过解析
#[test]
fn xml_dispatcher_handles_unclosed_tool_call() {
    // 构造包含未闭合标签的响应
    let response = ChatResponse {
        text: Some("Before\n<tool_call>\n{\"name\": \"shell\"}".into()),
        tool_calls: vec![],
        usage: None,
        reasoning_content: None,
    };

    let dispatcher = XmlToolDispatcher;
    let (text, calls) = dispatcher.parse_response(&response);

    // 未闭合标签不应触发 panic，仅作为文本处理
    assert!(calls.is_empty());
    assert!(text.contains("Before"));
}

/// 测试 NativeToolDispatcher 的结果格式化和 ID 映射
///
/// 验证调度器能将工具执行结果正确映射到对应的工具调用 ID。
/// Native 格式使用 `ToolResults` 消息类型，每个结果都关联到特定的工具调用。
///
/// # 测试场景
///
/// - 多个工具调用结果，每个都有唯一 ID
/// - 验证结果按 ID 正确映射
#[test]
fn native_format_results_maps_tool_call_ids() {
    let dispatcher = NativeToolDispatcher;

    // 创建带有工具调用 ID 的测试结果
    let results = vec![
        build_tool_execution_result("a", "out1", true, Some("tc-001")),
        build_tool_execution_result("b", "out2", true, Some("tc-002")),
    ];

    // 格式化结果
    let msg = dispatcher.format_results(&results);

    match msg {
        ConversationMessage::ToolResults(r) => {
            // 验证结果数量和 ID 映射
            assert_eq!(r.len(), 2);
            assert_eq!(r[0].tool_call_id, "tc-001");
            assert_eq!(r[0].content, "out1");
            assert_eq!(r[1].tool_call_id, "tc-002");
            assert_eq!(r[1].content, "out2");
        }
        _ => panic!("Expected ToolResults"),
    }
}

/// 测试 XmlToolDispatcher 将历史记录转换为提供商消息
///
/// 验证调度器能将完整的对话历史（包括系统消息、用户消息、工具调用和结果）
/// 转换为提供商 API 所需的消息格式。
///
/// # 转换规则
///
/// - 工具调用和结果被转换为用户/助手消息格式
/// - 保留原始对话结构
#[test]
fn xml_dispatcher_converts_history_to_provider_messages() {
    let dispatcher = XmlToolDispatcher;

    // 构造完整的对话历史
    let history = vec![
        ConversationMessage::Chat(ChatMessage::system("sys")),
        ConversationMessage::Chat(ChatMessage::user("hi")),
        ConversationMessage::AssistantToolCalls {
            text: Some("checking".into()),
            tool_calls: vec![ToolCall {
                kind: Default::default(),
                namespace: None,
                id: "tc1".into(),
                name: "shell".into(),
                arguments: "{}".into(),
            }],
            reasoning_content: None,
        },
        ConversationMessage::ToolResults(vec![ToolResultMessage {
            tool_call_id: "tc1".into(),
            content: "ok".into(),
        }]),
        ConversationMessage::Chat(ChatMessage::assistant("done")),
    ];

    // 转换为提供商消息格式
    let messages = dispatcher.to_provider_messages(&history);

    // 验证消息序列：system, user, assistant (工具调用), user (工具结果), assistant
    assert!(messages.len() >= 4);
    assert_eq!(messages[0].role, "system");
    assert_eq!(messages[1].role, "user");
}

/// 测试 NativeToolDispatcher 将工具结果转换为工具消息
///
/// 验证调度器能将工具执行结果转换为 `role: "tool"` 的消息格式，
/// 这是支持函数调用的模型提供商所需的标准格式。
#[test]
fn native_dispatcher_converts_tool_results_to_tool_messages() {
    let dispatcher = NativeToolDispatcher;

    // 构造工具结果历史
    let history = vec![ConversationMessage::ToolResults(vec![
        ToolResultMessage { tool_call_id: "tc1".into(), content: "output1".into() },
        ToolResultMessage { tool_call_id: "tc2".into(), content: "output2".into() },
    ])];

    // 转换为提供商消息
    let messages = dispatcher.to_provider_messages(&history);

    // 验证每个工具结果都被转换为独立的工具消息
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].role, "tool");
    assert_eq!(messages[1].role, "tool");
}

/// 测试 XmlToolDispatcher 生成工具使用提示指令
///
/// 验证调度器能为模型生成详细的工具使用说明，
/// 包括 XML 格式规范和可用工具列表。
///
/// # 提示内容
///
/// - 工具使用协议说明
/// - XML 标签格式示例
/// - 所有可用工具的描述
#[test]
fn xml_dispatcher_generates_tool_instructions() {
    let tools: Vec<Box<dyn crate::app::agent::tools::Tool>> = vec![Box::new(EchoTool)];
    let dispatcher = XmlToolDispatcher;
    let instructions = dispatcher.prompt_instructions(&tools);

    // 验证生成的指令包含必要信息
    assert!(instructions.contains("## Tool Use Protocol")); // 协议标题
    assert!(instructions.contains("<tool_call>")); // XML 标签格式
    assert!(instructions.contains("echo")); // 工具名称
    assert!(instructions.contains("Echoes the input")); // 工具描述
}

/// 测试 NativeToolDispatcher 不生成提示指令
///
/// 验证 Native 格式的调度器返回空字符串作为提示指令，
/// 因为它依赖提供商的原生工具调用机制，不需要额外的提示指导。
#[test]
fn native_dispatcher_returns_empty_instructions() {
    let tools: Vec<Box<dyn crate::app::agent::tools::Tool>> = vec![Box::new(EchoTool)];
    let dispatcher = NativeToolDispatcher;
    let instructions = dispatcher.prompt_instructions(&tools);

    // Native 调度器依赖工具规范而非提示指令
    assert!(instructions.is_empty());
}
