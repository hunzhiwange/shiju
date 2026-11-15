//! 运行请求标识生成。

use super::{
    AtomicU64,
    Ordering,
    SystemTime,
    UNIX_EPOCH,
};

pub(super) fn next_stream_request_id() -> String {
    static NEXT_SEQUENCE: AtomicU64 = AtomicU64::new(1);
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or_default();
    let sequence = NEXT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("tui-{now_ms}-{sequence}")
}

pub(super) fn next_plan_run_request_id() -> String {
    format!("plan-run-{}", next_stream_request_id())
}

pub(super) fn next_workflow_run_request_id() -> String {
    format!("workflow-run-{}", next_stream_request_id())
}

