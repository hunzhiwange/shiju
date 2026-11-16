use std::task::Poll;
#[test]
fn block_on_drives_pending_future_without_existing_runtime() {
    let mut first_poll = true;
    let value = crate::prefetch::block_on(std::future::poll_fn(move |cx| {
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

#[test]
fn app_session_scope_from_root_ignores_missing_or_blank_roots() {
    assert_eq!(crate::prefetch::app_session_scope_from_root(None), None);
    assert_eq!(crate::prefetch::app_session_scope_from_root(Some("   ")), None);
}

#[test]
fn app_session_scope_from_root_rejects_missing_directory() {
    let workspace = tempfile::tempdir().expect("temp workspace should be created");
    let missing = workspace.path().join("missing-project");

    assert_eq!(crate::prefetch::app_session_scope_from_root(Some(missing.to_string_lossy().as_ref())), None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn block_on_runs_future_inside_existing_runtime() {
    let value = crate::prefetch::block_on(async {
        tokio::task::yield_now().await;
        "inside-runtime".to_string()
    });

    assert_eq!(value, "inside-runtime");
}
