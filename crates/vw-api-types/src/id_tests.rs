//! 验证标识符和租户、项目作用域的序列化及非空约束。

use crate::id::{
    COMPANY_ID_HEADER, CompanyId, DEFAULT_COMPANY_ID, DEFAULT_PLATFORM_ID, PLATFORM_ID_HEADER,
    PlatformId, ProjectId, ProjectScope, SessionId, TenantScope,
};
use serde_json::json;

#[test]
fn string_ids_convert_and_serialize_as_plain_strings() {
    let project = ProjectId::from("project-1");
    let session = SessionId::from(String::from("session-1"));

    assert_eq!(project.as_ref(), "project-1");
    assert_eq!(session.as_ref(), "session-1");
    assert_eq!(serde_json::to_string(&project).expect("serialize"), "\"project-1\"");
}

#[test]
fn tenant_scope_default_uses_seeded_ids() {
    let tenant = TenantScope::default();

    assert_eq!(DEFAULT_PLATFORM_ID, "10000");
    assert_eq!(DEFAULT_COMPANY_ID, "100000000");
    assert_eq!(tenant.platform_id().as_ref(), DEFAULT_PLATFORM_ID);
    assert_eq!(tenant.company_id().as_ref(), DEFAULT_COMPANY_ID);
    assert_eq!(TenantScope::default(), tenant);
    assert_eq!(PLATFORM_ID_HEADER, "X-Platform-ID");
    assert_eq!(COMPANY_ID_HEADER, "X-Company-ID");
}

#[test]
fn validated_ids_reject_blank_values_from_constructors_and_json() {
    let platform_error = PlatformId::new(" \n ").expect_err("blank platform id should fail");
    assert_eq!(platform_error.to_string(), "platform id must not be empty");

    let company_error = CompanyId::new("").expect_err("empty company id should fail");
    assert_eq!(company_error.to_string(), "company id must not be empty");

    let deserialize_error = serde_json::from_value::<PlatformId>(json!("\t"))
        .expect_err("blank serialized platform id should fail");
    assert!(deserialize_error.to_string().contains("platform id must not be empty"));
}

#[test]
fn project_scope_validates_project_id_and_round_trips_accessors() {
    let tenant = TenantScope::new("platform-1", "company-1").expect("valid tenant scope");
    let scope = ProjectScope::new(tenant.clone(), ProjectId::from("project-1"))
        .expect("valid project scope");

    assert_eq!(scope.tenant_scope(), &tenant);
    assert_eq!(scope.platform_id().as_ref(), "platform-1");
    assert_eq!(scope.company_id().as_ref(), "company-1");
    assert_eq!(scope.project_id().as_ref(), "project-1");

    let constructor_error = ProjectScope::for_default_tenant(ProjectId::from("  "))
        .expect_err("blank project id should fail");
    assert_eq!(constructor_error.to_string(), "project id must not be empty");

    let deserialize_error = serde_json::from_value::<ProjectScope>(json!({
        "platform_id": "platform-1",
        "company_id": "company-1",
        "project_id": "\n"
    }))
    .expect_err("blank serialized project id should fail");
    assert!(deserialize_error.to_string().contains("project id must not be empty"));
}
