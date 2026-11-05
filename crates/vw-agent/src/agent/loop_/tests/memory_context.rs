//! 内存上下文相关功能的单元测试模块
//!
//! 本模块包含对代理循环中内存处理和凭据清洗功能的测试用例，
//! 主要涵盖以下功能领域：
//!
//! - **凭据清洗**：测试敏感信息（如API密钥、token、密码）的自动脱敏
//! - **历史记录裁剪**：测试对话历史的长度限制和系统提示保留
//! - **对话压缩**：测试长对话的摘要生成和替换机制
//! - **自动保存内存**：测试用户偏好的持久化存储
//! - **上下文构建**：测试从内存中构建查询上下文的逻辑
//! - **原生助手历史**：测试包含推理内容的助手消息构建

use super::*;
use crate::app::agent::memory::{Memory, MemoryCategory, SqliteMemory};
use tempfile::TempDir;

/// 测试凭据清洗功能 - 基本场景
///
/// 验证 scrub_credentials 函数能够正确识别并脱敏以下类型的敏感信息：
/// - API 密钥（格式：`key=value`）
/// - Token（格式：`token: value`）
/// - 密码（格式：`password="value"`）
///
/// 脱敏后的值应包含 `*[REDACTED]` 标记，且原始值不应出现在结果中。
#[test]
fn test_scrub_credentials() {
    let input = "API_KEY=sk-1234567890abcdef; token: 1234567890; password=\"secret123456\"";
    let scrubbed = scrub_credentials(input);
    assert!(scrubbed.contains("API_KEY=sk-1*[REDACTED]"));
    assert!(scrubbed.contains("token: 1234*[REDACTED]"));
    assert!(scrubbed.contains("password=\"secr*[REDACTED]\""));
    assert!(!scrubbed.contains("abcdef"));
    assert!(!scrubbed.contains("secret123456"));
}

/// 测试凭据清洗功能 - JSON 格式场景
///
/// 验证 scrub_credentials 函数能够正确处理 JSON 格式的敏感数据，
/// 确保键值对中的敏感值被脱敏，而其他公共数据保持不变。
#[test]
fn test_scrub_credentials_json() {
    let input = r#"{"api_key": "sk-1234567890", "other": "public"}"#;
    let scrubbed = scrub_credentials(input);
    assert!(scrubbed.contains("\"api_key\": \"sk-1*[REDACTED]\""));
    assert!(scrubbed.contains("public"));
}

/// 测试凭据清洗功能 - 空输入边缘情况
///
/// 验证当输入为空字符串时，scrub_credentials 函数返回空字符串，
/// 不应发生 panic 或返回错误结果。
#[test]
fn scrub_credentials_empty_input() {
    let result = scrub_credentials("");
    assert_eq!(result, "");
}

/// 测试凭据清洗功能 - 无敏感数据场景
///
/// 验证当输入文本中不包含任何敏感信息时，
/// scrub_credentials 函数应原样返回输入内容，不做任何修改。
#[test]
fn scrub_credentials_no_sensitive_data() {
    let input = "normal text without any secrets";
    let result = scrub_credentials(input);
    assert_eq!(result, input, "non-sensitive text should pass through unchanged");
}

/// 测试凭据清洗功能 - 短值不脱敏
///
/// 验证长度小于 8 个字符的值不会被脱敏处理。
/// 这是为了避免过度脱敏短字符串（如普通单词），
/// 只对足够长且可能是真正凭据的值进行脱敏。
#[test]
fn scrub_credentials_short_values_not_redacted() {
    let input = r#"api_key=\"short\""#;
    let result = scrub_credentials(input);
    assert_eq!(result, input, "short values should not be redacted");
}

/// 测试历史记录裁剪 - 系统提示保留
///
/// 验证 trim_history 函数在裁剪超长历史记录时：
/// 1. 始终保留第一位的系统提示消息
/// 2. 将历史记录裁剪到指定的最大消息数（系统提示不计入限制）
/// 3. 保留最近的消息，移除最旧的非系统消息
#[test]
fn trim_history_preserves_system_prompt() {
    // 构建包含系统提示和大量用户消息的历史记录
    let mut history = vec![ChatMessage::system("system prompt")];
    for i in 0..DEFAULT_MAX_HISTORY_MESSAGES + 20 {
        history.push(ChatMessage::user(format!("msg {i}")));
    }
    let original_len = history.len();
    assert!(original_len > DEFAULT_MAX_HISTORY_MESSAGES + 1);

    // 执行裁剪
    trim_history(&mut history, DEFAULT_MAX_HISTORY_MESSAGES);

    // 验证系统提示被保留
    assert_eq!(history[0].role, "system");
    assert_eq!(history[0].content, "system prompt");
    // 验证裁剪后的长度符合限制
    assert_eq!(history.len(), DEFAULT_MAX_HISTORY_MESSAGES + 1); // +1 for system
    // 验证保留了最近的消息
    let last = &history[history.len() - 1];
    assert_eq!(last.content, format!("msg {}", DEFAULT_MAX_HISTORY_MESSAGES + 19));
}

/// 测试历史记录裁剪 - 限制内无需裁剪
///
/// 验证当历史记录长度在限制范围内时，
/// trim_history 函数不应对历史记录进行任何修改。
#[test]
fn trim_history_noop_when_within_limit() {
    let mut history =
        vec![ChatMessage::system("sys"), ChatMessage::user("hello"), ChatMessage::assistant("hi")];
    trim_history(&mut history, DEFAULT_MAX_HISTORY_MESSAGES);
    assert_eq!(history.len(), 3);
}

/// 测试对话压缩转录格式化 - 角色标记
///
/// 验证 build_compaction_transcript 函数能够正确格式化对话消息，
/// 为用户消息添加 "USER:" 前缀，为助手消息添加 "ASSISTANT:" 前缀。
#[test]
fn build_compaction_transcript_formats_roles() {
    let messages = vec![ChatMessage::user("I like dark mode"), ChatMessage::assistant("Got it")];
    let transcript = build_compaction_transcript(&messages);
    assert!(transcript.contains("USER: I like dark mode"));
    assert!(transcript.contains("ASSISTANT: Got it"));
}

/// 测试应用压缩摘要 - 替换旧消息段
///
/// 验证 apply_compaction_summary 函数能够：
/// 1. 将指定范围的历史消息替换为压缩摘要
/// 2. 保留系统提示和最近的消息
/// 3. 摘要消息包含 "Compaction summary" 标识
#[test]
fn apply_compaction_summary_replaces_old_segment() {
    // 构建包含系统提示、旧消息和最近消息的历史记录
    let mut history = vec![
        ChatMessage::system("sys"),
        ChatMessage::user("old 1"),
        ChatMessage::assistant("old 2"),
        ChatMessage::user("recent 1"),
        ChatMessage::assistant("recent 2"),
    ];

    // 应用压缩摘要：从索引1开始，替换2条消息（old 1 和 old 2）
    apply_compaction_summary(&mut history, 1, 3, "- user prefers concise replies");

    // 验证结果
    assert_eq!(history.len(), 4);
    assert!(history[1].content.contains("Compaction summary"));
    assert!(history[2].content.contains("recent 1"));
    assert!(history[3].content.contains("recent 2"));
}

/// 测试自动保存内存键生成 - 前缀和唯一性
///
/// 验证 autosave_memory_key 函数生成的键：
/// 1. 包含指定的前缀并添加下划线后缀
/// 2. 每次调用生成的键都是唯一的（基于时间戳或其他机制）
#[test]
fn autosave_memory_key_has_prefix_and_uniqueness() {
    let key1 = autosave_memory_key("user_msg");
    let key2 = autosave_memory_key("user_msg");

    assert!(key1.starts_with("user_msg_"));
    assert!(key2.starts_with("user_msg_"));
    assert_ne!(key1, key2);
}

/// 测试自动保存内存键 - 多轮对话持久化
///
/// 验证使用不同键名可以保存多轮对话信息，
/// 且后续可以通过语义检索召回这些信息。
/// 使用 SqliteMemory 作为后端存储进行异步测试。
#[tokio::test]
async fn autosave_memory_keys_preserve_multiple_turns() {
    // 创建临时目录和 SQLite 内存存储
    let tmp = TempDir::new().unwrap();
    let mem = SqliteMemory::new(tmp.path()).unwrap();

    // 生成两个唯一的键
    let key1 = autosave_memory_key("user_msg");
    let key2 = autosave_memory_key("user_msg");

    // 存储两条不同的用户信息
    mem.store(&key1, "I'm Paul", MemoryCategory::Conversation, None).await.unwrap();
    mem.store(&key2, "I'm 45", MemoryCategory::Conversation, None).await.unwrap();

    // 验证两条记录都被存储
    assert_eq!(mem.count().await.unwrap(), 2);

    // 验证可以通过语义检索召回信息
    let recalled = mem.recall("45", 5, None).await.unwrap();
    assert!(recalled.iter().any(|entry| entry.content.contains("45")));
}

/// 测试上下文构建 - 忽略旧的助手自动保存条目
///
/// 验证 build_context 函数在构建上下文时会过滤掉
/// 以 "assistant_resp_" 为前缀的旧版自动保存条目，
/// 防止被污染的助手响应影响上下文构建。
///
/// 这是一个安全机制，确保只有用户消息和可信数据被纳入上下文。
#[tokio::test]
async fn build_context_ignores_legacy_assistant_autosave_entries() {
    // 创建临时目录和 SQLite 内存存储
    let tmp = TempDir::new().unwrap();
    let mem = SqliteMemory::new(tmp.path()).unwrap();

    // 存储一条被污染的旧版助手响应条目
    mem.store(
        "assistant_resp_poisoned",
        "User suffered a fabricated event",
        MemoryCategory::Daily,
        None,
    )
    .await
    .unwrap();

    // 存储一条正常的用户消息条目
    mem.store(
        "user_msg_real",
        "User asked for concise status updates",
        MemoryCategory::Conversation,
        None,
    )
    .await
    .unwrap();

    // 构建上下文并验证过滤结果
    let context = build_context(&mem, "status updates", 0.0).await;
    assert!(context.contains("user_msg_real"));
    assert!(!context.contains("assistant_resp_poisoned"));
    assert!(!context.contains("fabricated event"));
}

/// 测试历史记录裁剪边缘情况 - 空历史记录
///
/// 验证当历史记录为空时，trim_history 函数不应发生 panic，
/// 且历史记录应保持为空。
#[test]
fn trim_history_empty_history() {
    let mut history: Vec<crate::app::agent::providers::ChatMessage> = vec![];
    trim_history(&mut history, 10);
    assert!(history.is_empty());
}

/// 测试历史记录裁剪边缘情况 - 仅包含系统提示
///
/// 验证当历史记录仅包含系统提示时，
/// trim_history 函数应保留该系统提示，不进行任何移除。
#[test]
fn trim_history_system_only() {
    let mut history = vec![crate::app::agent::providers::ChatMessage::system("system prompt")];
    trim_history(&mut history, 10);
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].role, "system");
}

/// 测试历史记录裁剪边缘情况 - 恰好达到限制
///
/// 验证当历史记录恰好处于限制边界时（非系统消息数等于限制值），
/// trim_history 函数不应进行任何裁剪。
#[test]
fn trim_history_exactly_at_limit() {
    let mut history = vec![
        crate::app::agent::providers::ChatMessage::system("system"),
        crate::app::agent::providers::ChatMessage::user("msg 1"),
        crate::app::agent::providers::ChatMessage::assistant("reply 1"),
    ];
    trim_history(&mut history, 2); // 2 non-system messages = exactly at limit
    assert_eq!(history.len(), 3, "should not trim when exactly at limit");
}

/// 测试历史记录裁剪 - 移除最旧的非系统消息
///
/// 验证 trim_history 函数在需要裁剪时，
/// 会移除最旧的非系统消息，保留最新的消息。
#[test]
fn trim_history_removes_oldest_non_system() {
    let mut history = vec![
        crate::app::agent::providers::ChatMessage::system("system"),
        crate::app::agent::providers::ChatMessage::user("old msg"),
        crate::app::agent::providers::ChatMessage::assistant("old reply"),
        crate::app::agent::providers::ChatMessage::user("new msg"),
        crate::app::agent::providers::ChatMessage::assistant("new reply"),
    ];
    trim_history(&mut history, 2);
    assert_eq!(history.len(), 3); // system + 2 kept
    assert_eq!(history[0].role, "system");
    assert_eq!(history[1].content, "new msg");
}

/// 测试历史记录裁剪边缘情况 - 无系统提示的历史记录
///
/// 验证当历史记录中不包含系统提示时，
/// trim_history 函数仍能正确执行裁剪操作。
/// 这是一种恢复场景，确保系统对异常状态具有容错能力。
#[test]
fn trim_history_with_no_system_prompt() {
    // 构建不包含系统提示的历史记录
    let mut history = vec![];
    for i in 0..DEFAULT_MAX_HISTORY_MESSAGES + 20 {
        history.push(ChatMessage::user(format!("msg {i}")));
    }
    trim_history(&mut history, DEFAULT_MAX_HISTORY_MESSAGES);
    assert_eq!(history.len(), DEFAULT_MAX_HISTORY_MESSAGES);
}

/// 测试历史记录裁剪 - 保留角色顺序
///
/// 验证裁剪后历史记录中的角色顺序保持一致性。
/// 这是一种恢复场景，确保系统消息顺序在极端情况下仍然正确。
#[test]
fn trim_history_preserves_role_ordering() {
    // 构建包含系统提示和交替的用户/助手消息
    let mut history = vec![ChatMessage::system("system")];
    for i in 0..DEFAULT_MAX_HISTORY_MESSAGES + 10 {
        history.push(ChatMessage::user(format!("user {i}")));
        history.push(ChatMessage::assistant(format!("assistant {i}")));
    }
    trim_history(&mut history, DEFAULT_MAX_HISTORY_MESSAGES);

    // 验证第一条是系统消息，最后一条是助手消息
    assert_eq!(history[0].role, "system");
    assert_eq!(history[history.len() - 1].role, "assistant");
}

/// 测试历史记录裁剪边缘情况 - 仅系统提示不应被裁剪
///
/// 验证当历史记录仅包含一条系统提示时，
/// 即使限制为 0，系统提示也不应被移除。
/// 这是一种恢复场景，确保系统提示始终存在。
#[test]
fn trim_history_with_only_system_prompt() {
    let mut history = vec![ChatMessage::system("system prompt")];
    trim_history(&mut history, DEFAULT_MAX_HISTORY_MESSAGES);
    assert_eq!(history.len(), 1);
}

/// 测试构建原生助手历史 - 包含推理内容
///
/// 验证 build_native_assistant_history 函数能够：
/// 1. 构建包含 content、tool_calls 和 reasoning_content 的 JSON 消息
/// 2. 正确序列化推理内容（用于思维链功能）
/// 3. 正确序列化工具调用数组
#[test]
fn build_native_assistant_history_includes_reasoning_content() {
    // 构造工具调用
    let calls = vec![ToolCall {
        kind: Default::default(),
        namespace: None,
        id: "call_1".into(),
        name: "shell".into(),
        arguments: "{}".into(),
    }];

    // 构建包含推理内容的助手历史
    let result = build_native_assistant_history("answer", &calls, Some("thinking step"));

    // 解析并验证 JSON 结构
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["content"].as_str(), Some("answer"));
    assert_eq!(parsed["reasoning_content"].as_str(), Some("thinking step"));
    assert!(parsed["tool_calls"].is_array());
}

/// 测试构建原生助手历史 - 无推理内容时省略字段
///
/// 验证当推理内容为 None 时，
/// build_native_assistant_history 函数生成的 JSON 中不包含 reasoning_content 字段。
/// 这确保了消息格式的简洁性。
#[test]
fn build_native_assistant_history_omits_reasoning_content_when_none() {
    let calls = vec![ToolCall {
        kind: Default::default(),
        namespace: None,
        id: "call_1".into(),
        name: "shell".into(),
        arguments: "{}".into(),
    }];
    let result = build_native_assistant_history("answer", &calls, None);

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["content"].as_str(), Some("answer"));
    assert!(parsed.get("reasoning_content").is_none());
}

/// 测试从解析的工具调用构建原生助手历史 - 包含推理内容
///
/// 验证 build_native_assistant_history_from_parsed_calls 函数能够：
/// 1. 从 ParsedToolCall 结构构建消息
/// 2. 包含推理内容字段
/// 3. 返回 Some(String) 类型的 JSON 结果
#[test]
fn build_native_assistant_history_from_parsed_calls_includes_reasoning_content() {
    // 构造已解析的工具调用
    let calls = vec![ParsedToolCall {
        name: "shell".into(),
        arguments: serde_json::json!({"command": "pwd"}),
        tool_call_id: Some("call_2".into()),
    }];

    // 构建助手历史
    let result =
        build_native_assistant_history_from_parsed_calls("answer", &calls, Some("deep thought"));

    // 验证返回值存在
    assert!(result.is_some());

    // 解析并验证 JSON 结构
    let parsed: serde_json::Value = serde_json::from_str(result.as_deref().unwrap()).unwrap();
    assert_eq!(parsed["content"].as_str(), Some("answer"));
    assert_eq!(parsed["reasoning_content"].as_str(), Some("deep thought"));
    assert!(parsed["tool_calls"].is_array());
}

/// 测试从解析的工具调用构建原生助手历史 - 无推理内容时省略字段
///
/// 验证当推理内容为 None 时，
/// build_native_assistant_history_from_parsed_calls 函数生成的 JSON 中
/// 不包含 reasoning_content 字段，但其他字段（content、tool_calls）仍然正确。
#[test]
fn build_native_assistant_history_from_parsed_calls_omits_reasoning_content_when_none() {
    let calls = vec![ParsedToolCall {
        name: "shell".into(),
        arguments: serde_json::json!({"command": "pwd"}),
        tool_call_id: Some("call_2".into()),
    }];

    let result = build_native_assistant_history_from_parsed_calls("answer", &calls, None);

    // 验证返回值存在
    assert!(result.is_some());

    // 解析并验证 JSON 结构
    let parsed: serde_json::Value = serde_json::from_str(result.as_deref().unwrap()).unwrap();
    assert_eq!(parsed["content"].as_str(), Some("answer"));
    assert!(parsed.get("reasoning_content").is_none());
}
