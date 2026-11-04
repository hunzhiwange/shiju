//! 工具调用恢复路径测试。
//!
//! 本模块验证代理循环在重复工具调用、未执行工具却声称完成任务等
//! 异常模型响应下的防护行为。

use super::*;

#[tokio::test]
async fn run_tool_call_loop_executes_repaired_dsml_through_existing_permissions() {
    for native in [false, true] {
        for excluded in [false, true] {
            let mut provider = ScriptedProvider::from_text_responses(vec![
                "<｜｜DSML｜｜ calls><｜｜DSML｜｜ invoke name=\"count_tool\"><｜｜DSML｜｜ parameter name=\"value\" string=\"true\">repaired</｜｜DSML｜｜ parameter></｜｜DSML｜｜ invoke></｜｜DSML｜｜ calls>",
                "done",
            ]);
            if native {
                provider = provider.with_native_tool_support();
            }
            let invocations = Arc::new(AtomicUsize::new(0));
            let tools: Vec<Box<dyn Tool>> =
                vec![Box::new(CountingTool::new("count_tool", Arc::clone(&invocations)))];
            let mut history = vec![ChatMessage::system("test-system"), ChatMessage::user("run")];
            let excluded_tools = if excluded { vec!["count_tool".to_string()] } else { vec![] };
            let result = run_tool_call_loop(
                &provider,
                &mut history,
                &tools,
                &NoopObserver,
                "mock-provider",
                "mock-model",
                0.0,
                true,
                None,
                "cli",
                &crate::app::agent::config::MultimodalConfig::default(),
                3,
                None,
                None,
                None,
                None,
                &excluded_tools,
            )
            .await
            .expect("repaired tool calls should use the normal execution path");
            assert_eq!(result, "done");
            assert_eq!(invocations.load(Ordering::SeqCst), usize::from(!excluded));
            if excluded {
                assert!(
                    history.iter().any(|m| m.content.contains("not available in this channel"))
                );
            } else {
                assert!(history.iter().any(|m| m.content.contains("counted:repaired")));
                if native {
                    assert!(history.iter().any(|m| m.content.contains("fallback_1_1")));
                }
            }
        }
    }
}

#[tokio::test]
async fn run_tool_call_loop_rejects_malformed_dsml_without_final_text() {
    let malformed = "<｜｜DSML｜｜ calls><｜｜DSML｜｜ invoke name=\"Bash\">";
    for (with_tools, max_iterations) in [(false, 3), (true, 1), (true, 3)] {
        let provider = ScriptedProvider::from_text_responses(vec![malformed, malformed]);
        let invocations = Arc::new(AtomicUsize::new(0));
        let tools: Vec<Box<dyn Tool>> = if with_tools {
            vec![Box::new(CountingTool::new("count_tool", Arc::clone(&invocations)))]
        } else {
            vec![]
        };
        let mut history = vec![ChatMessage::system("test-system"), ChatMessage::user("run")];
        let error = run_tool_call_loop(
            &provider,
            &mut history,
            &tools,
            &NoopObserver,
            "mock-provider",
            "mock-model",
            0.0,
            true,
            None,
            "cli",
            &crate::app::agent::config::MultimodalConfig::default(),
            max_iterations,
            None,
            None,
            None,
            None,
            &[],
        )
        .await
        .expect_err("malformed DSML must never become a final answer");
        assert!(error.to_string().contains("unparseable tool call"), "{error}");
        assert_eq!(invocations.load(Ordering::SeqCst), 0);
    }
}

#[tokio::test]
async fn run_tool_call_loop_deduplicates_repeated_tool_calls() {
    let provider = ScriptedProvider::from_text_responses(vec![
        r#"<tool_call>
{"name":"count_tool","arguments":{"value":"A"}}
</tool_call>
<tool_call>
{"name":"count_tool","arguments":{"value":"A"}}
</tool_call>"#,
        "done",
    ])
    .with_native_tool_support();

    let invocations = Arc::new(AtomicUsize::new(0));
    let tools_registry: Vec<Box<dyn Tool>> =
        vec![Box::new(CountingTool::new("count_tool", Arc::clone(&invocations)))];

    let mut history = vec![ChatMessage::system("test-system"), ChatMessage::user("run tool calls")];
    let observer = NoopObserver;

    // 同一轮中完全相同的工具调用只应执行一次；第二条结果写入跳过信息，
    // 既避免重复副作用，也让模型知道该调用已被去重。
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
    .expect("loop should finish after deduplicating repeated calls");

    assert_eq!(result, "done");
    assert_eq!(
        invocations.load(Ordering::SeqCst),
        1,
        "duplicate tool call with same args should not execute twice"
    );

    let tool_results: Vec<_> = history.iter().filter(|msg| msg.role == "tool").collect();
    assert_eq!(tool_results.len(), 2, "native mode should emit one tool message per result");
    assert!(tool_results[0].content.contains("counted:A"));
    assert!(tool_results[1].content.contains("Skipped duplicate tool call"));
    assert!(
        history
            .iter()
            .all(|msg| !(msg.role == "user" && msg.content.starts_with("[Tool results]"))),
        "native mode should not emit prompt-style tool results"
    );
}

#[tokio::test]
async fn run_tool_call_loop_retries_when_response_claims_completion_without_tool_call() {
    let provider = ScriptedProvider::from_text_responses(vec![
        "Done — I've created the `names` folder in the current working directory.",
        r#"<tool_call>
{"name":"count_tool","arguments":{"value":"mkdir names"}}
</tool_call>"#,
        "done after verified tool execution",
    ]);

    let invocations = Arc::new(AtomicUsize::new(0));
    let tools_registry: Vec<Box<dyn Tool>> =
        vec![Box::new(CountingTool::new("count_tool", Arc::clone(&invocations)))];

    let mut history = vec![
        ChatMessage::system("test-system"),
        ChatMessage::user("please create the names folder"),
    ];
    let observer = NoopObserver;

    // 当模型声称已经完成文件系统类副作用但没有发出工具调用时，循环会
    // 追加恢复提示再给模型一次机会，防止“口头完成”掩盖实际未执行。
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
        5,
        None,
        None,
        None,
        None,
        &[],
    )
    .await
    .expect("completion claim without tool call should trigger a recovery retry");

    assert_eq!(result, "done after verified tool execution");
    assert_eq!(
        invocations.load(Ordering::SeqCst),
        1,
        "recovery retry should enforce one real tool execution"
    );
}

#[tokio::test]
async fn run_tool_call_loop_errors_when_completion_claim_repeats_without_tool_call() {
    let provider = ScriptedProvider::from_text_responses(vec![
        "Done — I've created the `names` folder in the current working directory.",
        "Finished successfully. The folder and file are now created in workspace.",
    ]);

    let invocations = Arc::new(AtomicUsize::new(0));
    let tools_registry: Vec<Box<dyn Tool>> =
        vec![Box::new(CountingTool::new("count_tool", Arc::clone(&invocations)))];

    let mut history = vec![
        ChatMessage::system("test-system"),
        ChatMessage::user("please create the names folder"),
    ];
    let observer = NoopObserver;

    // 连续声称完成但仍不调用工具时必须硬失败，避免上层把未验证的
    // 副作用当作真实结果展示给用户。
    let err = run_tool_call_loop(
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
        5,
        None,
        None,
        None,
        None,
        &[],
    )
    .await
    .expect_err("repeated completion claims without tool call should hard-fail");

    let err_text = err.to_string();
    assert!(
        err_text.contains("deferred action without emitting a tool call"),
        "unexpected error text: {err_text}"
    );
    assert_eq!(
        invocations.load(Ordering::SeqCst),
        0,
        "tool should not execute when provider never emits a real tool call"
    );
}

#[test]
fn looks_like_unverified_action_completion_without_tool_call_detects_claimed_side_effects() {
    // 这些语句看起来已经对工作区产生副作用，因此需要工具调用证据。
    assert!(looks_like_unverified_action_completion_without_tool_call(
        "Done — I've created the `names` folder in the current working directory."
    ));
    assert!(looks_like_unverified_action_completion_without_tool_call(
        "Finished successfully: I wrote the file to the workspace path."
    ));
}

#[test]
fn looks_like_unverified_action_completion_without_tool_call_ignores_non_side_effect_text() {
    // 解释或建议类回复不代表外部状态变化，不应触发恢复分支。
    assert!(!looks_like_unverified_action_completion_without_tool_call(
        "Done. Here is the explanation of why that approach works."
    ));
    assert!(!looks_like_unverified_action_completion_without_tool_call(
        "I have a suggestion for the plan if you want me to proceed."
    ));
}
