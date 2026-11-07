use crate::app::state::app_providers::paths::Form;
use vw_gateway_client::{
    AgentAppsReadError,
    vw_api_types::{
        agent_apps::AppId,
        app_providers::paths::{ProviderDirectoryStatus, ProviderPaths},
    },
};

#[test]
fn cc04_ui_paths_validation_failure_preserves_draft() {
    let mut form = Form { draft: "/授权外/目录".into(), pending: Some(9), ..Form::default() };
    form.finish(9, Err(AgentAppsReadError::Http { status: 422, body: serde_json::Value::Null }));
    assert_eq!(form.draft, "/授权外/目录");
    assert!(form.pending.is_none());
    assert!(form.error.is_some());
}

#[test]
fn cc04_ui_paths_stale_response_is_ignored() {
    let mut form =
        Form { draft: "~/新的配置目录".into(), pending: Some(9), ..Form::default() };
    form.finish(
        8,
        Ok(ProviderDirectoryStatus {
            paths: ProviderPaths {
                app_id: AppId::Claude,
                directory: "/fixture/旧目录".into(),
                data_directory: "/fixture/旧目录".into(),
                files: vec![],
            },
            directory_override: None,
        }),
    );
    assert_eq!(form.pending, Some(9));
    assert!(form.paths.is_none());
    assert_eq!(form.draft, "~/新的配置目录");
}
