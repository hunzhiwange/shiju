//! Host composition rejects missing project scope before IO.
use super::*;
fn assert_no_project<T>(result: Result<T, Error>) { assert!(matches!(result, Err(Error::NoProjectContext))); }
#[tokio::test]
// 上下文绑定接口统一在任何存储或事件副作用前检查项目作用域。
async fn context_bound_operations_reject_missing_project_before_side_effects() {
    assert_no_project(create_next(CreateInput::default()).await);
    assert_no_project(get("missing-session").await);
    assert_no_project(touch("missing-session").await);
    assert_no_project(
        update("missing-session", |_| {
            panic!("update callback must not run without project context");
        })
        .await,
    );
    assert_no_project(list().await);
    assert_no_project(children("missing-parent").await);
    assert_no_project(remove("missing-session").await);
    assert_no_project(fork("missing-session", None).await);
}
