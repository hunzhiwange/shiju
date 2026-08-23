//! 测试本地分派请求完整保留任务、运行、工作代理与作用域字段。
//! 克隆、相等比较和字段提取共同锁定该纯数据适配层的值语义。

use crate::agent_scheduler::adapters::local::LocalDispatchRequest;
use crate::agent_scheduler::domain::{RunId, TaskId, WorkerName};
use vw_api_types::common::TimestampMs;

fn local_dispatch_request(
    task_id: &str,
    run_id: &str,
    worker_id: &str,
    requested_at_ms: i64,
) -> LocalDispatchRequest {
    LocalDispatchRequest {
        task_id: TaskId::new(task_id).expect("task id should be valid"),
        run_id: RunId::new(run_id).expect("run id should be valid"),
        worker_id: WorkerName::new(worker_id).expect("worker name should be valid"),
        requested_at_ms: TimestampMs(requested_at_ms),
    }
}

#[test]
fn local_dispatch_request_preserves_fields() {
    let request = local_dispatch_request("task-1", "run-1", "worker-1", 123);

    assert_eq!(request.task_id.as_str(), "task-1");
    assert_eq!(request.run_id.as_str(), "run-1");
    assert_eq!(request.worker_id.as_str(), "worker-1");
    assert_eq!(request.requested_at_ms, TimestampMs(123));
}

#[test]
fn local_dispatch_request_supports_owned_field_extraction() {
    let LocalDispatchRequest { task_id, run_id, worker_id, requested_at_ms } =
        local_dispatch_request("task-1", "run-1", "worker-1", 123);

    assert_eq!(task_id.as_str(), "task-1");
    assert_eq!(run_id.as_str(), "run-1");
    assert_eq!(worker_id.as_str(), "worker-1");
    assert_eq!(requested_at_ms, TimestampMs(123));
}

#[test]
fn local_dispatch_request_supports_clone_and_equality() {
    fn assert_eq_trait<T: Eq>() {}

    assert_eq_trait::<LocalDispatchRequest>();

    let request = local_dispatch_request("task-1", "run-1", "worker-1", 123);
    let same = request.clone();

    assert_eq!(same, request);
}

#[test]
fn local_dispatch_request_equality_includes_every_field() {
    let request = local_dispatch_request("task-1", "run-1", "worker-1", 123);

    assert_ne!(local_dispatch_request("task-2", "run-1", "worker-1", 123), request);
    assert_ne!(local_dispatch_request("task-1", "run-2", "worker-1", 123), request);
    assert_ne!(local_dispatch_request("task-1", "run-1", "worker-2", 123), request);
    assert_ne!(local_dispatch_request("task-1", "run-1", "worker-1", 124), request);
}

#[test]
fn local_dispatch_request_debug_includes_type_and_fields() {
    let request = local_dispatch_request("task-1", "run-1", "worker-1", 123);
    let debug = format!("{request:?}");

    assert!(debug.contains("LocalDispatchRequest"));
    assert!(debug.contains("task_id"));
    assert!(debug.contains("run_id"));
    assert!(debug.contains("worker_id"));
    assert!(debug.contains("requested_at_ms"));
    assert!(debug.contains("task-1"));
    assert!(debug.contains("run-1"));
    assert!(debug.contains("worker-1"));
    assert!(debug.contains("123"));
}
