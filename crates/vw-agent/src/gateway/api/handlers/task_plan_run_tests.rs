//! `task_plan_run` 网关 handler 校验与路由接线测试。

use super::{Host, global_plan_run_control_registry, router};
use axum::{Extension, extract::Path};
use axum::Json;
use axum::http::StatusCode;
use vw_api_transport::id::TenantScope;
use vw_api_task::{TaskPlanRunStatus, TaskPlanRunStreamEventDto};

#[test]
fn task_plan_run_router_is_wired() {
    let _router = router::<()>();
}


#[tokio::test]
async fn task_plan_run_control_routes_are_tenant_scoped_and_stop_is_idempotent() {
    let owner = TenantScope::new("platform-route-owner", "company-route-owner").expect("owner");
    let outsider =
        TenantScope::new("platform-route-outsider", "company-route-outsider").expect("outsider");
    let task_id = format!("route-control-{}", uuid::Uuid::new_v4());
    let session_id = format!("{task_id}-plan");
    let registry = global_plan_run_control_registry();
    let cancellation =
        registry.register(owner.clone(), &task_id).expect("register route control test");
    registry
        .observe_event(
            &owner,
            &task_id,
            &TaskPlanRunStreamEventDto::PlanningStarted {
                task_id: task_id.clone(),
                plan_dir: "/repo/.plans/control".to_string(),
                session_id: session_id.clone(),
            },
        )
        .expect("observe planning");

    let Json(status) = task_plan_run_status::<Host>(Extension(owner.clone()), Path(task_id.clone()))
        .await
        .expect("owner status");
    assert_eq!(status.status, TaskPlanRunStatus::Running);
    assert_eq!(status.current_session_id.as_deref(), Some(session_id.as_str()));

    let cross_tenant_status =
        task_plan_run_status::<Host>(Extension(outsider.clone()), Path(task_id.clone()))
            .await
            .expect_err("cross-tenant status must be hidden");
    assert_eq!(cross_tenant_status.status, StatusCode::NOT_FOUND);
    let cross_tenant_stop = task_plan_run_stop::<Host>(Extension(outsider), Path(task_id.clone()))
        .await
        .expect_err("cross-tenant stop must be hidden");
    assert_eq!(cross_tenant_stop.status, StatusCode::NOT_FOUND);
    assert!(!cancellation.is_cancelled());

    let Json(stopped) = task_plan_run_stop::<Host>(Extension(owner.clone()), Path(task_id.clone()))
        .await
        .expect("owner stop");
    assert_eq!(stopped.status, TaskPlanRunStatus::Cancelled);
    assert!(cancellation.is_cancelled());

    let Json(stopped_again) =
        task_plan_run_stop::<Host>(Extension(owner), Path(task_id)).await.expect("idempotent stop");
    assert_eq!(stopped_again.status, TaskPlanRunStatus::Cancelled);
}
