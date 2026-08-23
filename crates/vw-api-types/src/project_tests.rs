//! 验证项目请求默认值、Git 状态载荷及租户作用域约束。

use crate::id::ProjectId;
use crate::project::{
    ListProjectsRequest, ProjectGitStateDto, ProjectScopeDto, ProjectStatus, TenantScopeDto,
    UpdateProjectRequest,
};
use serde_json::json;

// 列表和更新请求保持稀疏，项目状态及 Git 统计使用稳定线协议。
#[test]
fn project_requests_default_optional_fields() {
    let list: ListProjectsRequest = serde_json::from_value(json!({})).expect("valid list");
    assert_eq!(list.cursor, None);
    assert_eq!(list.limit, None);
    assert_eq!(list.status, None);

    let update: UpdateProjectRequest = serde_json::from_value(json!({})).expect("valid update");
    assert_eq!(update.name, None);
    assert_eq!(update.commands, None);

    assert_eq!(
        serde_json::to_value(ProjectStatus::Indexing).expect("serialize"),
        json!("indexing")
    );
    let git = ProjectGitStateDto {
        is_repo: true,
        has_uncommitted_changes: false,
        ahead: None,
        behind: Some(1),
    };
    assert_eq!(serde_json::to_value(git).expect("serialize")["behind"], json!(1));
}

// DTO 构造器只拒绝空作用域标识，不对有效标识施加额外字符限制。
#[test]
fn project_scope_dtos_reject_empty_tenant_ids() {
    assert!(TenantScopeDto::new("", "company-a").is_err());
    assert!(TenantScopeDto::new("platform-a", " ").is_err());

    let tenant = TenantScopeDto::new("platform/a", "company a").expect("tenant scope");
    let scope =
        ProjectScopeDto::new(tenant, ProjectId::from("project?one")).expect("project scope");
    assert_eq!(scope.platform_id().as_ref(), "platform/a");
    assert_eq!(scope.company_id().as_ref(), "company a");
    assert_eq!(scope.project_id().as_ref(), "project?one");
}
