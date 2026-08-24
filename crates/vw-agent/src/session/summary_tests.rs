//! 验证会话摘要输入、文件差异聚合与压缩提示生成行为。
//! 摘要既可来自消息内差异，也可来自持久化补丁统计；不完整快照必须回退而非虚构数据。

use super::*;
use crate::project::instance;
use crate::session::message;
use std::fs;

// 消息夹具显式携带摘要与补丁部件，用于区分两条差异聚合来源。
fn user_message_with_summary(
    id: &str,
    diffs: Vec<snapshot::FileDiff>,
    parts: Vec<message::Part>,
) -> message::WithParts {
    message::WithParts {
        info: message::Info::User(Box::new(message::UserInfo {
            id: id.to_string(),
            session_id: "ses_1".to_string(),
            time: message::UserTime { created: 1 },
            summary: Some(message::FileDiffSummary { title: None, body: None, diffs }),
            agent: "gateway".to_string(),
            model: message::ModelRef { provider_id: String::new(), model_id: String::new() },
            system: None,
            tools: None,
            variant: None,
        })),
        parts,
    }
}

fn modified_file_diff(file: &str, additions: i64, deletions: i64) -> snapshot::FileDiff {
    snapshot::FileDiff {
        file: file.to_string(),
        before: String::new(),
        after: String::new(),
        additions,
        deletions,
        status: Some(snapshot::DiffStatus::Modified),
    }
}

fn part_base(id: &str, message_id: &str) -> message::PartBase {
    message::PartBase {
        id: id.to_string(),
        session_id: "ses_1".to_string(),
        message_id: message_id.to_string(),
    }
}

fn token_info() -> message::TokenInfo {
    message::TokenInfo {
        total: None,
        input: 0,
        output: 0,
        reasoning: 0,
        cache: message::TokenCacheInfo { read: 0, write: 0 },
    }
}

fn assistant_message(id: &str, parent_id: &str) -> message::WithParts {
    message::WithParts {
        info: message::Info::Assistant(Box::new(message::AssistantInfo {
            id: id.to_string(),
            session_id: "ses_1".to_string(),
            time: message::AssistantTime {
                created: 1,
                completed: None,
                processing_elapsed_ms: None,
                llm_elapsed_ms: None,
            },
            error: None,
            parent_id: parent_id.to_string(),
            model_id: String::new(),
            provider_id: String::new(),
            mode: "primary".to_string(),
            agent: "gateway".to_string(),
            path: message::PathInfo { cwd: String::new(), root: String::new() },
            summary: None,
            cost: 0.0,
            tokens: token_info(),
            variant: None,
            finish: None,
        })),
        parts: Vec::new(),
    }
}

#[test]
fn inputs_use_gateway_field_names() {
    let summarize = serde_json::to_value(SummarizeInput {
        session_id: "s1".to_string(),
        message_id: "m1".to_string(),
    })
    .unwrap();
    assert_eq!(summarize["sessionID"], "s1");
    assert_eq!(summarize["messageID"], "m1");

    let diff =
        serde_json::to_value(DiffInput { session_id: "s1".to_string(), message_id: None }).unwrap();
    assert_eq!(diff["sessionID"], "s1");
    assert!(diff.get("messageID").is_none());
}

#[test]
fn inputs_deserialize_gateway_field_names_and_optional_message_id() {
    let summarize: SummarizeInput =
        serde_json::from_value(json!({ "sessionID": "s1", "messageID": "m1" })).unwrap();
    assert_eq!(summarize.session_id, "s1");
    assert_eq!(summarize.message_id, "m1");

    let diff: DiffInput =
        serde_json::from_value(json!({ "sessionID": "s1", "messageID": "m2" })).unwrap();
    assert_eq!(diff.session_id, "s1");
    assert_eq!(diff.message_id.as_deref(), Some("m2"));
}

#[test]
fn error_conversions_preserve_categories_and_display_messages() {
    let session: Error = crate::session::session::Error::NoProjectContext.into();
    let snapshot: Error = snapshot::Error::Git("invalid snapshot".to_string()).into();
    let llm: Error =
        crate::session::llm::Error::ProviderNotFound("missing-provider".to_string()).into();
    let storage: Error =
        crate::app::agent::storage::Error::NotFound(crate::app::agent::storage::NotFoundError {
            message: "missing record".to_string(),
        })
        .into();

    assert!(matches!(&session, Error::Session(_)));
    assert!(matches!(&snapshot, Error::Snapshot(_)));
    assert!(matches!(&llm, Error::Llm(_)));
    assert!(matches!(&storage, Error::Session(crate::session::session::Error::Storage(_))));
    assert_eq!(session.to_string(), "no active project context");
    assert_eq!(snapshot.to_string(), "invalid snapshot");
    assert_eq!(llm.to_string(), "provider not found: missing-provider");
    assert_eq!(storage.to_string(), "missing record");
}

#[test]
fn file_diffs_from_patch_parts_counts_unique_files() {
    let messages = vec![message::WithParts {
        info: message::Info::User(Box::new(message::UserInfo {
            id: "msg_1".to_string(),
            session_id: "ses_1".to_string(),
            time: message::UserTime { created: 1 },
            summary: None,
            agent: "gateway".to_string(),
            model: message::ModelRef { provider_id: String::new(), model_id: String::new() },
            system: None,
            tools: None,
            variant: None,
        })),
        parts: vec![
            message::Part::Patch(message::PatchPart {
                base: message::PartBase {
                    id: "prt_1".to_string(),
                    session_id: "ses_1".to_string(),
                    message_id: "msg_1".to_string(),
                },
                hash: "tree_1".to_string(),
                files: vec!["src/main.rs".to_string(), "src/lib.rs".to_string()],
                additions: None,
                deletions: None,
            }),
            message::Part::Patch(message::PatchPart {
                base: message::PartBase {
                    id: "prt_2".to_string(),
                    session_id: "ses_1".to_string(),
                    message_id: "msg_1".to_string(),
                },
                hash: "tree_2".to_string(),
                files: vec!["src/main.rs".to_string()],
                additions: None,
                deletions: None,
            }),
        ],
    }];

    let diffs = file_diffs_from_patch_parts(&messages);

    assert_eq!(diffs.len(), 2);
    assert!(diffs.iter().any(|diff| diff.file == "src/main.rs"));
    assert!(diffs.iter().any(|diff| diff.file == "src/lib.rs"));
    assert!(diffs.iter().all(|diff| {
        diff.before.is_empty()
            && diff.after.is_empty()
            && diff.additions == 0
            && diff.deletions == 0
            && diff.status == Some(snapshot::DiffStatus::Modified)
    }));
}

#[test]
fn file_diffs_from_patch_parts_ignores_other_parts() {
    let messages = vec![user_message_with_summary(
        "msg_1",
        Vec::new(),
        vec![message::Part::StepStart(message::StepStartPart {
            base: part_base("prt_1", "msg_1"),
            snapshot: None,
        })],
    )];

    assert!(file_diffs_from_patch_parts(&messages).is_empty());
}

#[test]
fn file_diffs_from_message_summaries_sums_changed_files() {
    let messages = vec![
        user_message_with_summary(
            "msg_1",
            vec![modified_file_diff("src/main.rs", 2, 1), modified_file_diff("src/lib.rs", 1, 0)],
            Vec::new(),
        ),
        user_message_with_summary(
            "msg_2",
            vec![modified_file_diff("src/main.rs", 3, 2)],
            vec![message::Part::Patch(message::PatchPart {
                base: message::PartBase {
                    id: "prt_1".to_string(),
                    session_id: "ses_1".to_string(),
                    message_id: "msg_2".to_string(),
                },
                hash: "tree_1".to_string(),
                files: vec!["src/main.rs".to_string(), "src/new.rs".to_string()],
                additions: None,
                deletions: None,
            })],
        ),
    ];

    let diffs = file_diffs_from_message_summaries(&messages);

    let main = diffs.iter().find(|diff| diff.file == "src/main.rs").unwrap();
    let lib = diffs.iter().find(|diff| diff.file == "src/lib.rs").unwrap();
    let new = diffs.iter().find(|diff| diff.file == "src/new.rs").unwrap();
    assert_eq!(main.additions, 5);
    assert_eq!(main.deletions, 3);
    assert_eq!(lib.additions, 1);
    assert_eq!(lib.deletions, 0);
    assert_eq!(new.additions, 0);
    assert_eq!(new.deletions, 0);
}

#[test]
fn file_diffs_from_message_summaries_skips_assistants_and_missing_summaries() {
    let mut without_summary = user_message_with_summary(
        "msg_1",
        vec![modified_file_diff("ignored.rs", 1, 1)],
        Vec::new(),
    );
    let message::Info::User(user) = &mut without_summary.info else { unreachable!() };
    user.summary = None;

    let diffs =
        file_diffs_from_message_summaries(&[assistant_message("msg_2", "msg_1"), without_summary]);

    assert!(diffs.is_empty());
}

#[test]
fn summary_from_diffs_totals_counts_and_file_count() {
    let summary = summary_from_diffs(&[
        modified_file_diff("src/main.rs", 4, 2),
        modified_file_diff("src/lib.rs", 1, 3),
    ]);

    assert_eq!(summary.additions, 5);
    assert_eq!(summary.deletions, 5);
    assert_eq!(summary.files, 2);
    assert!(summary.diffs.is_none());
}

#[test]
fn summary_from_empty_diffs_returns_zero_totals() {
    let summary = summary_from_diffs(&[]);

    assert_eq!(summary.additions, 0);
    assert_eq!(summary.deletions, 0);
    assert_eq!(summary.files, 0);
    assert!(summary.diffs.is_none());
}

#[test]
fn summary_from_persisted_patch_stats_accumulates_all_rounds_and_unique_files() {
    let mut first = assistant_message("msg_2", "msg_1");
    first.parts = vec![message::Part::Patch(message::PatchPart {
        base: part_base("prt_1", "msg_2"),
        hash: "tree_1".to_string(),
        files: vec!["src/main.rs".to_string(), "src/lib.rs".to_string()],
        additions: Some(5),
        deletions: Some(2),
    })];
    let mut second = assistant_message("msg_4", "msg_3");
    second.parts = vec![message::Part::Patch(message::PatchPart {
        base: part_base("prt_2", "msg_4"),
        hash: "tree_2".to_string(),
        files: vec!["src/main.rs".to_string()],
        additions: Some(3),
        deletions: Some(4),
    })];

    let summary =
        summary_from_persisted_patch_stats(&[first, second]).expect("persisted patch stats");

    assert_eq!(summary.additions, 8);
    assert_eq!(summary.deletions, 6);
    assert_eq!(summary.files, 2);
    assert!(summary.diffs.is_none());
}

#[test]
fn summary_from_persisted_patch_stats_ignores_legacy_or_incomplete_stats() {
    let mut legacy = assistant_message("msg_2", "msg_1");
    legacy.parts = vec![message::Part::Patch(message::PatchPart {
        base: part_base("prt_1", "msg_2"),
        hash: "tree_1".to_string(),
        files: vec!["src/main.rs".to_string()],
        additions: None,
        deletions: None,
    })];
    let mut incomplete = assistant_message("msg_4", "msg_3");
    incomplete.parts = vec![message::Part::Patch(message::PatchPart {
        base: part_base("prt_2", "msg_4"),
        hash: "tree_2".to_string(),
        files: vec!["src/lib.rs".to_string()],
        additions: Some(9),
        deletions: None,
    })];

    assert!(summary_from_persisted_patch_stats(&[legacy, incomplete]).is_none());
}

#[test]
fn instance_paths_use_safe_fallbacks_without_project_context() {
    assert_eq!(instance_directory_opt(), None);
    assert_eq!(worktree_path(), PathBuf::from("."));
}

#[test]
fn merge_file_diff_accumulates_and_preserves_useful_fields() {
    let mut diffs = vec![snapshot::FileDiff {
        file: "src/main.rs".to_string(),
        before: String::new(),
        after: "old".to_string(),
        additions: 1,
        deletions: 2,
        status: None,
    }];

    merge_file_diff(
        &mut diffs,
        &snapshot::FileDiff {
            file: "src/main.rs".to_string(),
            before: "before".to_string(),
            after: "after".to_string(),
            additions: 3,
            deletions: 4,
            status: Some(snapshot::DiffStatus::Modified),
        },
    );

    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].additions, 4);
    assert_eq!(diffs[0].deletions, 6);
    assert_eq!(diffs[0].before, "before");
    assert_eq!(diffs[0].after, "after");
    assert_eq!(diffs[0].status, Some(snapshot::DiffStatus::Modified));
}

#[test]
fn merge_file_diff_preserves_existing_fields_and_appends_new_files() {
    let mut diffs = vec![snapshot::FileDiff {
        file: "src/main.rs".to_string(),
        before: "original-before".to_string(),
        after: "original-after".to_string(),
        additions: 1,
        deletions: 2,
        status: Some(snapshot::DiffStatus::Added),
    }];

    merge_file_diff(
        &mut diffs,
        &snapshot::FileDiff {
            file: "src/main.rs".to_string(),
            before: "ignored-before".to_string(),
            after: String::new(),
            additions: 3,
            deletions: 4,
            status: Some(snapshot::DiffStatus::Deleted),
        },
    );
    merge_file_diff(&mut diffs, &modified_file_diff("src/lib.rs", 5, 6));

    assert_eq!(diffs.len(), 2);
    assert_eq!(diffs[0].before, "original-before");
    assert_eq!(diffs[0].after, "original-after");
    assert_eq!(diffs[0].additions, 4);
    assert_eq!(diffs[0].deletions, 6);
    assert_eq!(diffs[0].status, Some(snapshot::DiffStatus::Added));
    assert_eq!(diffs[1].file, "src/lib.rs");
}

// Git 的带引号路径可能含八进制转义；异常字节应保留可诊断文本而不是导致摘要失败。
#[test]
fn unquote_git_path_decodes_octal_common_escapes_and_leaves_plain_text() {
    assert_eq!(unquote_git_path(r#""caf\303\251.txt""#), "café.txt");
    assert_eq!(unquote_git_path(r#""dir\nfile\t\"quoted\".md""#), "dir\nfile\t\"quoted\".md");
    assert_eq!(unquote_git_path("plain.txt"), "plain.txt");
}

#[test]
fn unquote_git_path_handles_dangling_unknown_and_invalid_utf8_escapes() {
    assert_eq!(unquote_git_path(r#""trail\\""#), "trail\\");
    assert_eq!(unquote_git_path(r#""\z\b\f\v""#), "z\u{0008}\u{000c}\u{000b}");
    assert_eq!(unquote_git_path(r#""bad\377utf8""#), "bad�utf8");
    assert_eq!(unquote_git_path(r#""""#), "");
}

#[test]
fn unquote_git_path_handles_trimmed_quotes_octal_widths_and_carriage_return() {
    assert_eq!(unquote_git_path("  \"spaced path\"  "), "spaced path");
    assert_eq!(unquote_git_path(r#""\1\12\1018""#), "\u{0001}\nA8");
    assert_eq!(unquote_git_path(r#""line\rslash\\end""#), "line\rslash\\end");
    assert_eq!(unquote_git_path("\"trail\\\""), "trail\\");
    assert_eq!(unquote_git_path("\""), "\"");
}

#[test]
fn unquote_git_path_preserves_unquoted_whitespace_and_decodes_literal_unicode() {
    assert_eq!(unquote_git_path("  plain.txt  "), "  plain.txt  ");
    assert_eq!(unquote_git_path(r#""目录/文件.rs""#), "目录/文件.rs");
}

#[test]
fn extra_builds_json_map_from_pairs() {
    let map = extra([("title", Value::String("Hello".to_string())), ("count", Value::from(2))]);

    assert_eq!(map.get("title").and_then(Value::as_str), Some("Hello"));
    assert_eq!(map.get("count").and_then(Value::as_i64), Some(2));
}

#[test]
fn extra_handles_empty_input_and_last_duplicate_value_wins() {
    let empty = extra::<0>([]);
    let duplicate = extra([("key", Value::from(1)), ("key", Value::from(2))]);

    assert!(empty.is_empty());
    assert_eq!(duplicate.len(), 1);
    assert_eq!(duplicate.get("key").and_then(Value::as_i64), Some(2));
}

// 无完整快照对时回退到消息摘要，避免把缺失基线误解释为零差异。
#[test]
fn compute_diff_uses_message_summary_and_patch_fallback_without_snapshots() {
    let messages = vec![user_message_with_summary(
        "msg_1",
        vec![modified_file_diff("src/main.rs", 2, 1)],
        vec![message::Part::Patch(message::PatchPart {
            base: message::PartBase {
                id: "prt_1".to_string(),
                session_id: "ses_1".to_string(),
                message_id: "msg_1".to_string(),
            },
            hash: "tree_1".to_string(),
            files: vec!["src/main.rs".to_string(), "src/extra.rs".to_string()],
            additions: None,
            deletions: None,
        })],
    )];

    let diffs = compute_diff(&messages).expect("diffs");

    assert!(diffs.iter().any(|diff| diff.file == "src/main.rs" && diff.additions == 2));
    assert!(diffs.iter().any(|diff| diff.file == "src/extra.rs" && diff.additions == 0));
}

#[test]
fn compute_diff_empty_messages_returns_no_diffs() {
    let diffs = compute_diff(&[]).expect("diffs");

    assert!(diffs.is_empty());
}

#[test]
fn compute_diff_incomplete_snapshot_pairs_use_message_summary_fallback() {
    let start_only = user_message_with_summary(
        "msg_1",
        vec![modified_file_diff("start.rs", 1, 0)],
        vec![message::Part::StepStart(message::StepStartPart {
            base: part_base("prt_1", "msg_1"),
            snapshot: Some("snapshot-start".to_string()),
        })],
    );
    let finish_only = user_message_with_summary(
        "msg_2",
        vec![modified_file_diff("finish.rs", 0, 1)],
        vec![message::Part::StepFinish(message::StepFinishPart {
            base: part_base("prt_2", "msg_2"),
            reason: "stop".to_string(),
            snapshot: Some("snapshot-finish".to_string()),
            cost: 0.0,
            tokens: token_info(),
        })],
    );

    let start_diffs = compute_diff(&[start_only]).expect("start fallback");
    let finish_diffs = compute_diff(&[finish_only]).expect("finish fallback");

    assert_eq!(start_diffs.len(), 1);
    assert_eq!(start_diffs[0].file, "start.rs");
    assert_eq!(finish_diffs.len(), 1);
    assert_eq!(finish_diffs[0].file, "finish.rs");
}

#[tokio::test]
async fn summarize_message_returns_ok_for_missing_message_without_side_effects() {
    summarize_message("session-missing", "message-missing", &[]).await.unwrap();
}

#[tokio::test]
async fn summarize_message_returns_ok_for_assistant_target_without_side_effects() {
    let assistant = assistant_message("msg_2", "msg_1");

    summarize_message("ses_1", "msg_2", &[assistant]).await.unwrap();
}

#[tokio::test]
async fn diff_missing_storage_returns_empty_list() {
    let diffs = diff(DiffInput {
        session_id: format!("missing-summary-diff-{}", std::process::id()),
        message_id: Some("msg".to_string()),
    })
    .await;

    assert!(diffs.is_empty());
}

#[tokio::test]
async fn diff_reads_unquotes_and_rewrites_persisted_diff_paths() {
    let session_id = format!("summary-diff-{}", std::process::id());
    let stored = vec![snapshot::FileDiff {
        file: r#""caf\303\251.txt""#.to_string(),
        before: String::new(),
        after: String::new(),
        additions: 1,
        deletions: 0,
        status: Some(snapshot::DiffStatus::Added),
    }];
    crate::app::agent::storage::write(&["session_diff", &session_id], &stored).await.unwrap();

    let diffs = diff(DiffInput { session_id: session_id.clone(), message_id: None }).await;

    assert_eq!(diffs[0].file, "café.txt");
    let rewritten =
        crate::app::agent::storage::read::<Vec<snapshot::FileDiff>>(&["session_diff", &session_id])
            .await
            .unwrap();
    assert_eq!(rewritten[0].file, "café.txt");
}

#[tokio::test]
async fn diff_recomputes_stale_empty_cache_from_session_messages() {
    let session_id = format!("summary-diff-stale-cache-{}", std::process::id());
    let message_id = format!("msg-stale-cache-{}", std::process::id());
    let part_id = format!("prt-stale-cache-{}", std::process::id());
    let info = message::Info::User(Box::new(message::UserInfo {
        id: message_id.clone(),
        session_id: session_id.clone(),
        time: message::UserTime { created: 1 },
        summary: None,
        agent: "gateway".to_string(),
        model: message::ModelRef { provider_id: String::new(), model_id: String::new() },
        system: None,
        tools: None,
        variant: None,
    }));
    let part = message::Part::Patch(message::PatchPart {
        base: message::PartBase {
            id: part_id,
            session_id: session_id.clone(),
            message_id,
        },
        hash: "tree-stale-cache".to_string(),
        files: vec!["demo/111.md".to_string()],
        additions: None,
        deletions: None,
    });

    message::update_message(&info).await.expect("persist message");
    message::update_part(&part).await.expect("persist patch part");
    storage::write(
        &["session_diff", &session_id],
        &Vec::<snapshot::FileDiff>::new(),
    )
    .await
    .expect("persist empty diff cache");

    let diffs = recompute_session_diff_for_read(&session_id)
        .await
        .expect("recompute persisted messages");

    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].file, "demo/111.md");

    let read_diffs = diff(DiffInput { session_id, message_id: None }).await;
    assert_eq!(read_diffs.len(), 1);
    assert_eq!(read_diffs[0].file, "demo/111.md");
}

// 端到端验证：工具创建未跟踪文件后，diff() 应回退到实时计算并返回 Added 状态。
#[tokio::test]
async fn diff_fallback_includes_untracked_file_as_added() {
    use crate::snapshot;

    // 临时 git 仓库需要系统 git 可用
    let _env_guard = {
        let previous = std::env::var_os("VIBEWINDOW_TEST_PREFER_SYSTEM_GIT");
        unsafe { std::env::set_var("VIBEWINDOW_TEST_PREFER_SYSTEM_GIT", "1") };
        struct Guard(Option<std::ffi::OsString>);
        impl Drop for Guard {
            fn drop(&mut self) {
                unsafe {
                    match &self.0 {
                        Some(v) => std::env::set_var("VIBEWINDOW_TEST_PREFER_SYSTEM_GIT", v),
                        None => std::env::remove_var("VIBEWINDOW_TEST_PREFER_SYSTEM_GIT"),
                    }
                }
            }
        }
        Guard(previous)
    };

    let dir = tempfile::tempdir().expect("temp dir");
    let worktree = dir.path();

    // 初始化临时 git 仓库
    crate::app::agent::shell::git_std_command()
        .current_dir(worktree)
        .args(["init"])
        .output()
        .expect("git init");
    crate::app::agent::shell::git_std_command()
        .current_dir(worktree)
        .args(["config", "user.name", "Test"])
        .output()
        .expect("git config name");
    crate::app::agent::shell::git_std_command()
        .current_dir(worktree)
        .args(["config", "user.email", "test@example.com"])
        .output()
        .expect("git config email");
    fs::write(worktree.join("initial.txt"), "initial\n").expect("write initial");
    crate::app::agent::shell::git_std_command()
        .current_dir(worktree)
        .args(["add", "."])
        .output()
        .expect("git add");
    crate::app::agent::shell::git_std_command()
        .current_dir(worktree)
        .args(["commit", "-m", "init"])
        .output()
        .expect("git commit");

    // 创建基线快照
    let baseline = snapshot::track(worktree).expect("track").expect("baseline hash");

    // 模拟工具创建未跟踪文件
    fs::write(worktree.join("untracked.txt"), "new content\n").expect("write untracked");

    // 创建结束快照（包含未跟踪文件）
    let after = snapshot::track(worktree).expect("track").expect("after hash");

    let session_id = format!("diff-fallback-{}", std::process::id());

    // 在 instance 上下文中执行，确保 worktree_path() 返回正确路径
    instance::provide(worktree, None, move || {
        Box::pin(async move {
            // 构造包含快照对的消息
            let user_msg = message::WithParts {
                info: message::Info::User(Box::new(message::UserInfo {
                    id: "msg_1".to_string(),
                    session_id: session_id.clone(),
                    time: message::UserTime { created: 1 },
                    summary: None,
                    agent: "gateway".to_string(),
                    model: message::ModelRef { provider_id: String::new(), model_id: String::new() },
                    system: None,
                    tools: None,
                    variant: None,
                })),
                parts: vec![
                    message::Part::StepStart(message::StepStartPart {
                        base: message::PartBase {
                            id: "part_start".to_string(),
                            session_id: session_id.clone(),
                            message_id: "msg_1".to_string(),
                        },
                        snapshot: Some(baseline),
                    }),
                    message::Part::StepFinish(message::StepFinishPart {
                        base: message::PartBase {
                            id: "part_finish".to_string(),
                            session_id: session_id.clone(),
                            message_id: "msg_1".to_string(),
                        },
                        reason: "stop".to_string(),
                        snapshot: Some(after),
                        cost: 0.0,
                        tokens: message::TokenInfo {
                            total: None,
                            input: 0,
                            output: 0,
                            reasoning: 0,
                            cache: message::TokenCacheInfo { read: 0, write: 0 },
                        },
                    }),
                ],
            };

            // 持久化消息及部件
            message::update_message(&user_msg.info).await.expect("persist message");
            for part in &user_msg.parts {
                message::update_part(part).await.expect("persist part");
            }

            // 故意不写入 session_diff 缓存，模拟缓存漏刷场景
            // 调用 diff() 应回退到 compute_diff_inner 并包含未跟踪文件
            let diffs = diff(DiffInput { session_id: session_id.clone(), message_id: None }).await;

            // 验证未跟踪文件被识别为 Added
            let untracked_diff = diffs
                .iter()
                .find(|d| d.file == "untracked.txt")
                .expect("untracked.txt should be in diffs");
            assert_eq!(untracked_diff.status, Some(snapshot::DiffStatus::Added));
            assert_eq!(untracked_diff.after, "new content\n");
            assert!(untracked_diff.additions > 0);
        })
    })
    .await
    .expect("instance provide");
}
