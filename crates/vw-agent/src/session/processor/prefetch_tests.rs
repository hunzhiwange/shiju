//! 验证文件上下文预取的权限、类型、去重与数量限制。

use crate::app::agent::session::session::{Role, Session};
use crate::app::agent::tools::ToolRuntimeContext;
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::Poll;

static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(0);

fn unique_session_id(name: &str) -> String {
    let sequence = NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed);
    format!("prefetch-tests-{name}-{}-{sequence}", std::process::id())
}

fn ctx_for(root: Option<String>) -> ToolRuntimeContext {
    ToolRuntimeContext::new(unique_session_id("ctx"), root)
}

fn read_allowed(path: &std::path::Path) -> bool {
    let target = path.to_string_lossy();
    super::prefetch_allowed("read", target.as_ref())
}

#[test]
fn block_on_drives_pending_future_without_existing_runtime() {
    let mut first_poll = true;
    let value = super::block_on(std::future::poll_fn(move |cx| {
        if first_poll {
            first_poll = false;
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(42)
        }
    }));

    assert_eq!(value, 42);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn block_on_runs_future_inside_existing_runtime() {
    let value = super::block_on(async {
        tokio::task::yield_now().await;
        "inside-runtime".to_string()
    });

    assert_eq!(value, "inside-runtime");
}

#[test]
fn app_session_scope_from_root_ignores_missing_or_blank_roots() {
    assert_eq!(super::app_session_scope_from_root(None), None);
    assert_eq!(super::app_session_scope_from_root(Some("   ")), None);
}

#[test]
fn app_session_scope_from_root_rejects_missing_directory() {
    let workspace = tempfile::tempdir().expect("temp workspace should be created");
    let missing = workspace.path().join("missing-project");

    assert_eq!(super::app_session_scope_from_root(Some(missing.to_string_lossy().as_ref())), None);
}

#[test]
fn try_prefetch_read_returns_false_for_missing_paths_and_directories() {
    let workspace = tempfile::tempdir().expect("temp workspace should be created");
    let ctx = ctx_for(Some(workspace.path().to_string_lossy().to_string()));
    let mut session = Session::new(ctx.session.clone());
    let mut tool_state = super::super::ToolSessionState::default();

    assert!(!super::try_prefetch_read(&mut session, "missing.rs", &ctx, &mut tool_state));
    assert!(!super::try_prefetch_read(&mut session, ".", &ctx, &mut tool_state));
    assert!(session.messages.is_empty());
    assert_eq!(tool_state.non_todo_tool_runs, 0);
}

#[test]
fn try_prefetch_read_records_text_file_only_when_permission_allows() {
    let workspace = tempfile::tempdir().expect("temp workspace should be created");
    let file = workspace.path().join("style.css");
    std::fs::write(&file, "body { color: black; }\n").expect("style.css should be written");
    let ctx = ctx_for(Some(workspace.path().to_string_lossy().to_string()));
    let mut session = Session::new(ctx.session.clone());
    let mut tool_state = super::super::ToolSessionState::default();
    let allowed = read_allowed(&file);

    let prefetched = super::try_prefetch_read(&mut session, "style.css", &ctx, &mut tool_state);

    assert_eq!(prefetched, allowed);
    if allowed {
        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.messages[0].role, Role::Tool);
        assert!(session.messages[0].content.contains("style.css"));
        assert_eq!(tool_state.non_todo_tool_runs, 1);
    } else {
        assert!(session.messages.is_empty());
        assert_eq!(tool_state.non_todo_tool_runs, 0);
    }
}

#[test]
fn try_prefetch_read_rejects_binary_files_without_recording_tools() {
    let workspace = tempfile::tempdir().expect("temp workspace should be created");
    std::fs::write(workspace.path().join("archive.bin"), [0_u8, 1, 2, 3])
        .expect("binary fixture should be written");
    let ctx = ctx_for(Some(workspace.path().to_string_lossy().to_string()));
    let mut session = Session::new(ctx.session.clone());
    let mut tool_state = super::super::ToolSessionState::default();

    assert!(!super::try_prefetch_read(&mut session, "archive.bin", &ctx, &mut tool_state,));
    assert!(session.messages.is_empty());
    assert_eq!(tool_state.non_todo_tool_runs, 0);
}

#[test]
fn prefetch_seed_context_skips_when_query_contains_explicit_tool_call() {
    let workspace = tempfile::tempdir().expect("temp workspace should be created");
    std::fs::write(workspace.path().join("style.css"), "body { color: black; }\n")
        .expect("style.css should be written");
    let ctx = ctx_for(Some(workspace.path().to_string_lossy().to_string()));
    let mut session = Session::new(ctx.session.clone());
    let mut tool_state = super::super::ToolSessionState::default();
    let allowed = HashSet::from(["file_read".to_string()]);

    super::prefetch_seed_context(
        &mut session,
        r#"/file_read {"path":"style.css"}"#,
        &ctx,
        &allowed,
        &mut tool_state,
    );

    assert!(session.messages.is_empty());
    assert_eq!(tool_state.non_todo_tool_runs, 0);
}

#[test]
fn prefetch_seed_context_deduplicates_repeated_file_candidates() {
    let workspace = tempfile::tempdir().expect("temp workspace should be created");
    let file = workspace.path().join("style.css");
    std::fs::write(&file, "body { color: black; }\n").expect("style.css should be written");
    let ctx = ctx_for(Some(workspace.path().to_string_lossy().to_string()));
    let mut session = Session::new(ctx.session.clone());
    let mut tool_state = super::super::ToolSessionState::default();
    let expected_runs = if read_allowed(&file) { 1 } else { 0 };

    super::prefetch_seed_context(
        &mut session,
        "Compare style.css with style.css, then inspect style.css again",
        &ctx,
        &HashSet::new(),
        &mut tool_state,
    );

    assert_eq!(session.messages.len(), expected_runs);
    assert_eq!(tool_state.non_todo_tool_runs, expected_runs as u64);
}

#[test]
fn prefetch_seed_context_limits_prefetch_to_first_three_candidates() {
    let workspace = tempfile::tempdir().expect("temp workspace should be created");
    let names = ["one.rs", "two.rs", "three.rs", "four.rs"];
    for name in names {
        std::fs::write(workspace.path().join(name), format!("// {name}\n"))
            .expect("text fixture should be written");
    }
    let ctx = ctx_for(Some(workspace.path().to_string_lossy().to_string()));
    let mut session = Session::new(ctx.session.clone());
    let mut tool_state = super::super::ToolSessionState::default();
    let expected_runs =
        names[..3].iter().filter(|name| read_allowed(&workspace.path().join(name))).count();

    super::prefetch_seed_context(
        &mut session,
        "Read one.rs, two.rs, three.rs, and four.rs",
        &ctx,
        &HashSet::new(),
        &mut tool_state,
    );

    assert_eq!(session.messages.len(), expected_runs);
    assert_eq!(tool_state.non_todo_tool_runs, expected_runs as u64);
}

#[test]
fn prefetch_seed_context_leaves_session_unchanged_without_file_candidates() {
    let workspace = tempfile::tempdir().expect("temp workspace should be created");
    let ctx = ctx_for(Some(workspace.path().to_string_lossy().to_string()));
    let mut session = Session::new(ctx.session.clone());
    let mut tool_state = super::super::ToolSessionState::default();

    super::prefetch_seed_context(
        &mut session,
        "Explain the current design",
        &ctx,
        &HashSet::new(),
        &mut tool_state,
    );

    assert!(session.messages.is_empty());
    assert_eq!(tool_state.non_todo_tool_runs, 0);
}

#[test]
fn prefetch_seed_context_ignores_http_urls_and_absent_files() {
    let workspace = tempfile::tempdir().expect("temp workspace should be created");
    let ctx = ctx_for(Some(workspace.path().to_string_lossy().to_string()));
    let mut session = Session::new(ctx.session.clone());
    let mut tool_state = super::super::ToolSessionState::default();

    super::prefetch_seed_context(
        &mut session,
        "Please compare https://example.com/style.css with src/missing.rs and README.md",
        &ctx,
        &HashSet::new(),
        &mut tool_state,
    );

    assert!(session.messages.is_empty());
    assert_eq!(tool_state.non_todo_tool_runs, 0);
}
