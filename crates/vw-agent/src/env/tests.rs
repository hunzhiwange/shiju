// 覆盖进程环境快照的隔离、嵌套恢复与锁中毒回退。

use super::{all, get, provide, provide_with, remove, set};
use std::collections::HashMap;

// 这些用例会修改进程级环境变量，因此通过共享锁与守卫避免并发污染并恢复原值。
#[test]
fn injected_environment_is_isolated_from_default_snapshot() {
    let mut vars = HashMap::new();
    vars.insert("VIBEWINDOW_TEST_KEY".to_string(), "one".to_string());

    provide_with(vars, || {
        assert_eq!(get("VIBEWINDOW_TEST_KEY").as_deref(), Some("one"));
        set("VIBEWINDOW_TEST_KEY", "two");
        assert_eq!(get("VIBEWINDOW_TEST_KEY").as_deref(), Some("two"));
        remove("VIBEWINDOW_TEST_KEY");
        assert_eq!(get("VIBEWINDOW_TEST_KEY"), None);
        assert!(!all().contains_key("VIBEWINDOW_TEST_KEY"));
    });
}

#[test]
fn provide_captures_process_environment_and_restores_outer_context() {
    let mut outer = HashMap::new();
    outer.insert("VIBEWINDOW_OUTER".to_string(), "outer".to_string());
    let process_path = std::env::var("PATH").ok();

    provide_with(outer, || {
        assert_eq!(get("VIBEWINDOW_OUTER").as_deref(), Some("outer"));

        let inner_result = provide(|| {
            assert_eq!(get("VIBEWINDOW_OUTER"), None);
            if let Some(path) = process_path.as_deref() {
                assert_eq!(get("PATH").as_deref(), Some(path));
            }
            set("VIBEWINDOW_INNER", "changed");
            get("VIBEWINDOW_INNER")
        });

        assert_eq!(inner_result.as_deref(), Some("changed"));
        assert_eq!(get("VIBEWINDOW_OUTER").as_deref(), Some("outer"));
        assert_eq!(get("VIBEWINDOW_INNER"), None);
    });
}

#[test]
fn default_context_can_be_updated_and_cleared() {
    let key = "VIBEWINDOW_DEFAULT_SNAPSHOT_TEST";

    set(key, "context");
    assert_eq!(get(key).as_deref(), Some("context"));
    set(key, "changed-context");
    assert_eq!(get(key).as_deref(), Some("changed-context"));

    remove(key);
    assert_eq!(get(key), None);
}

#[test]
fn injected_environment_allows_empty_snapshot() {
    provide_with(HashMap::new(), || {
        assert_eq!(get("VIBEWINDOW_EMPTY_ENV_TEST"), None);
        assert!(all().is_empty());

        set("VIBEWINDOW_EMPTY_ENV_TEST", "created");
        assert_eq!(all().get("VIBEWINDOW_EMPTY_ENV_TEST").map(String::as_str), Some("created"));
    });
}

#[test]
fn poisoned_environment_lock_returns_safe_fallbacks() {
    let mut vars = HashMap::new();
    vars.insert("VIBEWINDOW_POISONED_ENV_TEST".to_string(), "before".to_string());

    provide_with(vars, || {
        let env = super::ENV_CONTEXT.use_value().expect("env context exists");
        let poisoned = std::thread::spawn(move || {
            let _guard = env.lock().expect("env lock");
            panic!("poison env lock");
        })
        .join();

        assert!(poisoned.is_err());
        assert_eq!(get("VIBEWINDOW_POISONED_ENV_TEST"), None);
        assert!(all().is_empty());

        set("VIBEWINDOW_POISONED_ENV_TEST", "after");
        remove("VIBEWINDOW_POISONED_ENV_TEST");
        assert_eq!(get("VIBEWINDOW_POISONED_ENV_TEST"), None);
    });
}
