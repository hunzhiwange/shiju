//! 原生工具调用模式测试。
//!
//! 本模块验证 native tool 模式下的历史记录形状、工具调用 id 保留，
//! 以及系统提示中不会混入 XML fallback 协议。

use super::*;

#[tokio::test]
async fn run_tool_call_loop_native_mode_preserves_fallback_tool_call_ids() {
    let provider = ScriptedProvider::from_text_responses(vec![
        r#"{"content":"Need to call tool","tool_calls":[{"id":"call_abc","name":"count_tool","arguments":"{\"value\":\"X\"}"}]}"#,
        "done",
    ])
    .with_native_tool_support();

    let invocations = Arc::new(AtomicUsize::new(0));
    let tools_registry: Vec<Box<dyn Tool>> =
        vec![Box::new(CountingTool::new("count_tool", Arc::clone(&invocations)))];

    let mut history = vec![ChatMessage::system("test-system"), ChatMessage::user("run tool calls")];
    let observer = NoopObserver;

    // 这里的 provider 以文本形式返回 JSON 工具调用；native 模式需要兼容
    // 这种 fallback 解析结果，并保留上游传入的 tool_call_id 供后续关联。
    let result = run_tool_call_loop(
        &provider,
        &mut history,
        &tools_registry,
        &observer,
        "mock-provider",
        "mock-model",
        0.0,
        true,
        None,
        "cli",
        &crate::app::agent::config::MultimodalConfig::default(),
        4,
        None,
        None,
        None,
        None,
        &[],
    )
    .await
    .expect("native fallback id flow should complete");

    assert_eq!(result, "done");
    assert_eq!(invocations.load(Ordering::SeqCst), 1);
    assert!(
        history.iter().any(|msg| {
            msg.role == "tool" && msg.content.contains("\"tool_call_id\":\"call_abc\"")
        }),
        "tool result should preserve parsed fallback tool_call_id in native mode"
    );
    assert!(
        history
            .iter()
            .all(|msg| !(msg.role == "user" && msg.content.starts_with("[Tool results]"))),
        "native mode should use role=tool history instead of prompt fallback wrapper"
    );
}

#[test]
fn native_tools_system_prompt_contains_zero_xml() {
    use crate::app::agent::channels::build_system_prompt_with_mode;

    // 原生工具调用由 provider API 承载，系统提示只保留任务与工具摘要，
    // 避免 XML 协议和 native schema 同时出现造成模型行为混乱。
    let tool_summaries: Vec<(&str, &str)> =
        vec![("bash", "Execute shell commands"), ("file_read", "Read files")];

    let system_prompt = build_system_prompt_with_mode(
        std::path::Path::new("/tmp"),
        "test-model",
        &tool_summaries,
        &[],
        None,
        None,
        true,
        crate::app::agent::config::SkillsPromptInjectionMode::Full,
        chrono_tz::Asia::Shanghai,
    );

    assert!(!system_prompt.contains("<tool_call>"), "Native prompt must not contain <tool_call>");
    assert!(!system_prompt.contains("</tool_call>"), "Native prompt must not contain </tool_call>");
    assert!(!system_prompt.contains("<tool_result>"), "原生模式提示不应包含 <tool_result> 标记");
    assert!(!system_prompt.contains("</tool_result>"), "原生模式提示不应包含 </tool_result> 标记");
    assert!(
        !system_prompt.contains("## Tool Use Protocol"),
        "原生模式提示不应包含 XML 协议说明章节"
    );

    assert!(system_prompt.contains("bash"), "原生模式提示应列出工具名称");
    assert!(system_prompt.contains("## Execution Style"), "原生模式提示应包含执行风格指令");
}
