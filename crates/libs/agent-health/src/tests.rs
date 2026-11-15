// 覆盖健康检查注册、执行及失败信息保留行为。

// 这组轻量测试确认成功与失败更新都会出现在下一次健康快照中。
use super::*;

#[test]
fn mark_component_ok_adds_component_to_snapshot() {
    mark_component_ok("unit-test-component");
    let snapshot = snapshot();

    assert!(
        snapshot
            .components
            .iter()
            .any(|(name, component)| name == "unit-test-component" && component.status == "ok")
    );
}

#[test]
fn mark_component_error_records_message() {
    mark_component_error("unit-test-error", "failed");
    let snapshot = snapshot();
    let component =
        snapshot.components.iter().find(|(name, _)| *name == "unit-test-error").expect("component");

    assert_eq!(component.1.status, "error");
    assert_eq!(component.1.last_error.as_deref(), Some("failed"));
}
