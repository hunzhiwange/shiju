// 验证健康状态聚合、组件检查结果与总体状态判定。

// 中毒锁恢复是运行时可用性边界，快照仍需按组件名稳定排序。
use super::*;

#[test]
fn component_lifecycle_preserves_last_ok_and_tracks_restarts() {
    let component = "mod-test-component-lifecycle";

    mark_component_ok(component);
    bump_component_restart(component);
    mark_component_error(component, "disk full");

    let snapshot = snapshot();
    let health = snapshot.components.get(component).expect("component health");

    assert_eq!(health.status, "error");
    assert!(health.last_ok.is_some());
    assert_eq!(health.last_error.as_deref(), Some("disk full"));
    assert_eq!(health.restart_count, 1);
    assert!(health.updated_at.contains('T'));
}

#[test]
fn mark_component_ok_clears_previous_error_and_refreshes_last_ok() {
    let component = "mod-test-component-recovery";

    mark_component_error(component, "temporary failure");
    mark_component_ok(component);

    let snapshot = snapshot();
    let health = snapshot.components.get(component).expect("component health");

    assert_eq!(health.status, "ok");
    assert!(health.last_ok.is_some());
    assert_eq!(health.last_error, None);
}

#[test]
fn bump_component_restart_creates_starting_component_and_saturates() {
    let component = "mod-test-component-restart-only";

    bump_component_restart(component);
    bump_component_restart(component);

    let snapshot = snapshot();
    let health = snapshot.components.get(component).expect("component health");

    assert_eq!(health.status, "starting");
    assert_eq!(health.restart_count, 2);
    assert_eq!(health.last_ok, None);
    assert_eq!(health.last_error, None);
}

#[test]
fn snapshot_contains_process_metadata_and_sorted_components() {
    mark_component_ok("mod-test-z");
    mark_component_ok("mod-test-a");

    let snapshot = snapshot();
    let keys: Vec<_> =
        snapshot.components.keys().filter(|key| key.starts_with("mod-test-")).cloned().collect();

    assert_eq!(snapshot.pid, std::process::id());
    assert!(snapshot.updated_at.contains('T'));
    assert!(snapshot.uptime_seconds < 24 * 60 * 60);
    assert!(keys.windows(2).all(|pair| pair[0] <= pair[1]));
}

#[test]
fn snapshot_json_serializes_health_snapshot() {
    mark_component_error("mod-test-json", "json failure");

    let json = snapshot_json();

    assert_eq!(json["pid"], std::process::id());
    assert_eq!(json["components"]["mod-test-json"]["status"], "error");
    assert_eq!(json["components"]["mod-test-json"]["last_error"], "json failure");
}

#[test]
fn registry_operations_recover_from_poisoned_component_lock() {
    let _ = std::panic::catch_unwind(|| {
        let _guard = registry().components.lock().unwrap();
        panic!("poison health registry lock for recovery test");
    });

    mark_component_ok("mod-test-poison-recovery");
    bump_component_restart("mod-test-poison-recovery");
    mark_component_error("mod-test-poison-recovery", "recovered");

    let snapshot = snapshot();
    let health = snapshot.components.get("mod-test-poison-recovery").expect("component health");

    assert_eq!(health.status, "error");
    assert_eq!(health.last_error.as_deref(), Some("recovered"));
    assert_eq!(health.restart_count, 1);
    assert_eq!(snapshot_json()["components"]["mod-test-poison-recovery"]["status"], "error");
}
