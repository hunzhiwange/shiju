//! 覆盖恢复选择器从旧式回滚文件与分页历史接口生成转录预览的规则。
//! 反向扫描必须遵守字节、记录和可见项预算，并在无法证明安全时明确回退。
//! 跨两种数据源的用例要求相同会话内容产生一致的用户/助手预览。

use super::*;
use crate::legacy_core::config::ConfigBuilder;
use app_test_support::create_mock_responses_server_sequence;
use app_test_support::write_mock_responses_config_toml;
use codex_app_server_client::AppServerEvent;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::ServerNotification;
use codex_app_server_protocol::SortDirection;
use codex_app_server_protocol::ThreadItemsListParams;
use codex_app_server_protocol::ThreadItemsListResponse;
use codex_app_server_protocol::ThreadStartParams;
use codex_app_server_protocol::ThreadStartResponse;
use codex_app_server_protocol::TurnStartParams;
use codex_app_server_protocol::TurnStartResponse;
use codex_app_server_protocol::UserInput;
use codex_protocol::protocol::AgentMessageEvent;
use codex_protocol::protocol::ThreadRolledBackEvent;
use codex_protocol::protocol::UserMessageEvent;
use codex_rollout::CompactedItem;
use core_test_support::responses;
use pretty_assertions::assert_eq;
use tempfile::tempdir;

fn preview_line(speaker: TranscriptPreviewSpeaker, text: &str) -> TranscriptPreviewLine {
    // 构造期望预览行，统一说话者与文本的比较形式。
    TranscriptPreviewLine {
        speaker,
        text: text.to_string(),
    }
}

fn rollout_user_message(text: &str) -> RolloutItem {
    // 生成旧式回滚日志中的用户消息记录。
    RolloutItem::EventMsg(EventMsg::UserMessage(UserMessageEvent {
        message: text.to_string(),
        ..Default::default()
    }))
}

fn rollout_agent_message(text: &str) -> RolloutItem {
    // 生成旧式回滚日志中的助手消息记录。
    RolloutItem::EventMsg(EventMsg::AgentMessage(AgentMessageEvent {
        message: text.to_string(),
        phase: None,
        memory_citation: None,
        delivery: None,
    }))
}

fn write_preview_rollout(path: &Path, items: Vec<RolloutItem>) {
    // 写入最小 JSONL 回滚文件，保留真实反向扫描的文件边界行为。
    let contents = items
        .into_iter()
        .map(|item| {
            serde_json::to_string(&RolloutLine {
                timestamp: String::from("2026-07-19T12:00:00Z"),
                ordinal: None,
                item,
            })
            .expect("serialize rollout line")
        })
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(path, format!("{contents}\n")).expect("write rollout");
}

#[test]
// 尾部包含压缩边界时，扫描应越过隐藏记录找到最近的可见对话。
fn legacy_transcript_preview_scans_tail_across_compaction() {
    let temp_dir = tempdir().expect("tempdir");
    let path = temp_dir.path().join("rollout.jsonl");
    write_preview_rollout(
        &path,
        vec![
            rollout_user_message("older user"),
            rollout_agent_message("older assistant"),
            RolloutItem::Compacted(CompactedItem {
                message: String::from("summary that is not transcript text"),
                replacement_history: None,
                mcp_resource_origins: None,
                window_number: None,
                first_window_id: None,
                previous_window_id: None,
                window_id: None,
            }),
            rollout_user_message("recent user"),
            rollout_agent_message("commentary one\ncommentary two"),
            rollout_agent_message("final one\n\n final two "),
        ],
    );

    let mut lines = scan_legacy_transcript_preview(
        &path,
        temp_dir.path(),
        /*inline_visualization_context*/ None,
    )
    .expect("scan rollout")
    .expect("bounded tail");
    lines.reverse();

    assert_eq!(
        lines,
        vec![
            preview_line(TranscriptPreviewSpeaker::Assistant, "older assistant"),
            preview_line(TranscriptPreviewSpeaker::User, "recent user"),
            preview_line(TranscriptPreviewSpeaker::Assistant, "commentary one"),
            preview_line(TranscriptPreviewSpeaker::Assistant, "commentary two"),
            preview_line(TranscriptPreviewSpeaker::Assistant, "final one"),
            preview_line(TranscriptPreviewSpeaker::Assistant, "final two"),
        ]
    );
}

#[test]
// 回滚标记落在尾部预算内会使新旧内容关系不确定，应改用完整回退路径。
fn legacy_transcript_preview_falls_back_when_rollback_is_in_the_tail() {
    let temp_dir = tempdir().expect("tempdir");
    let path = temp_dir.path().join("rollout.jsonl");
    write_preview_rollout(
        &path,
        vec![
            rollout_user_message("rolled back user"),
            rollout_agent_message("rolled back assistant"),
            RolloutItem::EventMsg(EventMsg::ThreadRolledBack(ThreadRolledBackEvent {
                num_turns: 1,
            })),
            rollout_user_message("recent user"),
            rollout_agent_message("recent assistant"),
        ],
    );

    let lines = scan_legacy_transcript_preview(
        &path,
        temp_dir.path(),
        /*inline_visualization_context*/ None,
    )
    .expect("scan rollout");

    assert_eq!(lines, None);
}

#[test]
// 回滚标记位于扫描窗口之外时，预览只采用其后的稳定记录。
fn legacy_transcript_preview_stops_before_rollback_outside_the_tail() {
    let temp_dir = tempdir().expect("tempdir");
    let path = temp_dir.path().join("rollout.jsonl");
    write_preview_rollout(
        &path,
        vec![
            rollout_user_message("rolled back user"),
            RolloutItem::EventMsg(EventMsg::ThreadRolledBack(ThreadRolledBackEvent {
                num_turns: 1,
            })),
            rollout_user_message("recent user"),
            rollout_agent_message("one\ntwo\nthree\nfour\nfive\nsix"),
        ],
    );

    let mut lines = scan_legacy_transcript_preview(
        &path,
        temp_dir.path(),
        /*inline_visualization_context*/ None,
    )
    .expect("scan rollout")
    .expect("bounded tail");
    lines.reverse();

    assert_eq!(
        lines,
        vec![
            preview_line(TranscriptPreviewSpeaker::Assistant, "one"),
            preview_line(TranscriptPreviewSpeaker::Assistant, "two"),
            preview_line(TranscriptPreviewSpeaker::Assistant, "three"),
            preview_line(TranscriptPreviewSpeaker::Assistant, "four"),
            preview_line(TranscriptPreviewSpeaker::Assistant, "five"),
            preview_line(TranscriptPreviewSpeaker::Assistant, "six"),
        ]
    );
}

#[test]
// 单条可见记录超过大小上限时不能局部截取，应请求可靠的后备读取。
fn legacy_transcript_preview_falls_back_for_oversized_visible_record() {
    let temp_dir = tempdir().expect("tempdir");
    let path = temp_dir.path().join("rollout.jsonl");
    write_preview_rollout(
        &path,
        vec![rollout_agent_message(
            &"x".repeat(MAX_LEGACY_TRANSCRIPT_PREVIEW_SCAN_BYTES),
        )],
    );

    let lines = scan_legacy_transcript_preview(
        &path,
        temp_dir.path(),
        /*inline_visualization_context*/ None,
    )
    .expect("scan rollout");

    assert_eq!(lines, None);
}

#[test]
// 即使超大记录最终不可见，也会破坏反向扫描边界，仍必须回退。
fn legacy_transcript_preview_falls_back_for_oversized_hidden_record() {
    let temp_dir = tempdir().expect("tempdir");
    let path = temp_dir.path().join("rollout.jsonl");
    write_preview_rollout(
        &path,
        vec![
            rollout_user_message("older user"),
            RolloutItem::Compacted(CompactedItem {
                message: "x".repeat(MAX_LEGACY_TRANSCRIPT_PREVIEW_SCAN_BYTES),
                replacement_history: None,
                mcp_resource_origins: None,
                window_number: None,
                first_window_id: None,
                previous_window_id: None,
                window_id: None,
            }),
            rollout_agent_message("recent assistant"),
        ],
    );

    let lines = scan_legacy_transcript_preview(
        &path,
        temp_dir.path(),
        /*inline_visualization_context*/ None,
    )
    .expect("scan rollout");

    assert_eq!(lines, None);
}

#[test]
// 在找到足够预览内容前耗尽扫描预算时，不能把不完整尾部当作最终结果。
fn legacy_transcript_preview_falls_back_when_scan_budget_is_exhausted() {
    let temp_dir = tempdir().expect("tempdir");
    let path = temp_dir.path().join("rollout.jsonl");
    let compacted = RolloutItem::Compacted(CompactedItem {
        message: "x".repeat(MAX_LEGACY_TRANSCRIPT_PREVIEW_SCAN_BYTES / 8),
        replacement_history: None,
        mcp_resource_origins: None,
        window_number: None,
        first_window_id: None,
        previous_window_id: None,
        window_id: None,
    });
    let mut items = vec![rollout_user_message("older user")];
    items.extend(std::iter::repeat_n(compacted, 9));
    items.push(rollout_agent_message("recent assistant"));
    write_preview_rollout(&path, items);

    let lines = scan_legacy_transcript_preview(
        &path,
        temp_dir.path(),
        /*inline_visualization_context*/ None,
    )
    .expect("scan rollout");

    assert_eq!(lines, None);
}

#[test]
// 已收集足够的新记录后应停止反向扫描，不再读取更旧项目。
fn transcript_preview_reverse_scan_stops_before_older_items() {
    let newest = ThreadItem::AgentMessage {
        id: String::from("final"),
        text: String::from("one\ntwo\nthree\nfour\nfive\nsix"),
        phase: None,
        memory_citation: None,
        delivery: None,
    };
    let mut lines = Vec::new();

    append_transcript_preview_lines(
        &mut lines,
        std::iter::once(&newest).chain(std::iter::from_fn(|| {
            panic!("preview should stop after finding six lines")
        })),
        Path::new("/tmp"),
        /*inline_visualization_context*/ None,
    );

    assert_eq!(
        lines,
        vec![
            preview_line(TranscriptPreviewSpeaker::Assistant, "six"),
            preview_line(TranscriptPreviewSpeaker::Assistant, "five"),
            preview_line(TranscriptPreviewSpeaker::Assistant, "four"),
            preview_line(TranscriptPreviewSpeaker::Assistant, "three"),
            preview_line(TranscriptPreviewSpeaker::Assistant, "two"),
            preview_line(TranscriptPreviewSpeaker::Assistant, "one"),
        ]
    );
}

async fn transcript_preview_for_history_mode(
    history_mode: ThreadHistoryMode,
    invisible_newer_turns: usize,
) -> Vec<TranscriptPreviewLine> {
    // 用统一入口运行旧式或分页模式，便于直接比较两条实现路径。
    let model_response = responses::sse(vec![
        responses::ev_response_created("resp-1"),
        responses::ev_assistant_message("commentary", "commentary one\ncommentary two"),
        responses::ev_assistant_message("final", "final one\nfinal two\nfinal three"),
        responses::ev_completed("resp-1"),
    ]);
    let model_responses = std::iter::once(model_response)
        .chain((0..invisible_newer_turns).map(|index| {
            responses::sse(vec![
                responses::ev_response_created(&format!("empty-resp-{index}")),
                responses::ev_completed(&format!("empty-resp-{index}")),
            ])
        }))
        .collect();
    let server = create_mock_responses_server_sequence(model_responses).await;
    let codex_home = tempdir().expect("tempdir");
    write_mock_responses_config_toml(
        codex_home.path(),
        &server.uri(),
        &Default::default(),
        /*auto_compact_limit*/ 100_000,
        /*requires_openai_auth*/ None,
        "mock_provider",
        "compact",
    )
    .expect("write mock config");
    let config = ConfigBuilder::default()
        .codex_home(codex_home.path().to_path_buf())
        .fallback_cwd(Some(codex_home.path().to_path_buf()))
        .build()
        .await
        .expect("build config");
    let mut app_server = crate::start_embedded_app_server_for_picker(&config)
        .await
        .expect("start app server");
    let request_handle = app_server.request_handle();
    let started: ThreadStartResponse = request_handle
        .request_typed(ClientRequest::ThreadStart {
            request_id: RequestId::String(String::from("preview-thread-start")),
            params: ThreadStartParams {
                model: Some(String::from("mock-model")),
                model_provider: Some(String::from("mock_provider")),
                cwd: Some(codex_home.path().display().to_string()),
                history_mode: Some(history_mode),
                ..Default::default()
            },
        })
        .await
        .expect("start thread");
    let thread_id = ThreadId::from_string(&started.thread.id).expect("thread id");
    for index in 0..=invisible_newer_turns {
        let input = if index == 0 {
            UserInput::Text {
                text: String::from("recent user"),
                text_elements: Vec::new(),
            }
        } else {
            UserInput::Image {
                url: String::from(
                    "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==",
                ),
                detail: None,
            }
        };
        let _: TurnStartResponse = request_handle
            .request_typed(ClientRequest::TurnStart {
                request_id: RequestId::String(format!("preview-turn-start-{index}")),
                params: TurnStartParams {
                    thread_id: thread_id.to_string(),
                    input: vec![input],
                    ..Default::default()
                },
            })
            .await
            .expect("start turn");
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                let Some(event) = app_server.next_event().await else {
                    panic!("app-server event stream closed before turn completed");
                };
                if matches!(
                    event,
                    AppServerEvent::ServerNotification(notification)
                        if matches!(notification.as_ref(), ServerNotification::TurnCompleted(_))
                ) {
                    break;
                }
            }
        })
        .await
        .expect("turn should complete");
    }

    if history_mode == ThreadHistoryMode::Paginated {
        let first_page: ThreadItemsListResponse = request_handle
            .request_typed(ClientRequest::ThreadItemsList {
                request_id: RequestId::String(String::from("preview-items-list")),
                params: ThreadItemsListParams {
                    thread_id: thread_id.to_string(),
                    turn_id: None,
                    cursor: None,
                    limit: Some(TRANSCRIPT_PREVIEW_ITEMS_PAGE_SIZE),
                    sort_direction: Some(SortDirection::Desc),
                },
            })
            .await
            .expect("list first items page");
        assert_eq!(first_page.data.len(), MAX_TRANSCRIPT_PREVIEW_LINES);
        let mut first_page_lines = Vec::new();
        append_transcript_preview_lines(
            &mut first_page_lines,
            first_page.data.iter().map(|entry| &entry.item),
            codex_home.path(),
            /*inline_visualization_context*/ None,
        );
        assert_eq!(first_page_lines, Vec::new());
        assert!(first_page.next_cursor.is_some());
    }

    load_transcript_preview(&mut app_server, thread_id, Some(&config))
        .await
        .expect("load transcript preview")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
// 相同回滚内容经旧式与分页读取后应得到相同的预览顺序与文本。
async fn transcript_preview_preserves_legacy_and_paginated_output() {
    let invisible_newer_turns = TRANSCRIPT_PREVIEW_ITEMS_PAGE_SIZE as usize;
    let legacy =
        transcript_preview_for_history_mode(ThreadHistoryMode::Legacy, invisible_newer_turns).await;
    let paginated =
        transcript_preview_for_history_mode(ThreadHistoryMode::Paginated, invisible_newer_turns)
            .await;

    assert_eq!(legacy, paginated);
    assert_eq!(
        paginated,
        vec![
            preview_line(TranscriptPreviewSpeaker::User, "recent user"),
            preview_line(TranscriptPreviewSpeaker::Assistant, "commentary one"),
            preview_line(TranscriptPreviewSpeaker::Assistant, "commentary two"),
            preview_line(TranscriptPreviewSpeaker::Assistant, "final one"),
            preview_line(TranscriptPreviewSpeaker::Assistant, "final two"),
            preview_line(TranscriptPreviewSpeaker::Assistant, "final three"),
        ]
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
// 最新页被大量无关旧项目占满时，分页实现应继续翻页找到可见对话。
async fn paginated_transcript_preview_pages_past_old_item_budget() {
    let invisible_newer_turns = TRANSCRIPT_PREVIEW_ITEMS_PAGE_SIZE as usize * 4;

    let lines =
        transcript_preview_for_history_mode(ThreadHistoryMode::Paginated, invisible_newer_turns)
            .await;

    assert_eq!(
        lines,
        vec![
            preview_line(TranscriptPreviewSpeaker::User, "recent user"),
            preview_line(TranscriptPreviewSpeaker::Assistant, "commentary one"),
            preview_line(TranscriptPreviewSpeaker::Assistant, "commentary two"),
            preview_line(TranscriptPreviewSpeaker::Assistant, "final one"),
            preview_line(TranscriptPreviewSpeaker::Assistant, "final two"),
            preview_line(TranscriptPreviewSpeaker::Assistant, "final three"),
        ]
    );
}
