use reqwest::StatusCode;
use serde_json::{Value, json};
use vw_api_transport::{ProjectId, TenantScope, USER_ID_HEADER, UserId};
use vw_api_types::cli_workbench::{CliWorkbenchSession, CliWorkbenchTarget};

use super::{ChildAccess, CliWorkbenchChildContext, WORKBENCH};
use crate::gateway::api::handlers::cli_workbench::{ProcessState, Session, capabilities};

#[tokio::test]
async fn cli_workbench_child_access_restricts_project_identity_and_routes() {
    let tenant = TenantScope::new("workbench-child", "fixture")
        .unwrap()
        .with_user_id(UserId::new("bound-user").unwrap());
    let project = "child-project";
    let session_id = "child-session";
    let scope = vw_api_transport::ProjectScope::new(tenant, ProjectId::from(project)).unwrap();
    let access = ChildAccess::start(&scope, session_id).await.unwrap();
    let credential: CliWorkbenchChildContext = serde_json::from_str(access.context()).unwrap();

    // Keep the fixture's launch binding alive only for the life of this test.
    struct Cleanup;
    impl Drop for Cleanup {
        fn drop(&mut self) {
            WORKBENCH.lock().sessions.remove("child-session");
        }
    }
    let _cleanup = Cleanup;
    let target = CliWorkbenchTarget::Acp { agent: "acp-fixture".to_owned() };
    WORKBENCH.lock().sessions.insert(
        session_id.to_owned(),
        Session {
            scope,
            info: CliWorkbenchSession {
                id: session_id.to_owned(),
                project_id: project.to_owned(),
                directory: "/fixture".to_owned(),
                capabilities: capabilities(&target),
                target,
                state: ProcessState::Running,
            },
            turn: None,
            #[cfg(target_os = "macos")]
            terminal: None,
            child_access: Some(access),
        },
    );

    let http = reqwest::Client::builder().no_proxy().build().unwrap();
    let base = format!("http://127.0.0.1:{}/v1", credential.port);
    let route = format!("{base}/projects/{project}/cli-workbench/sessions/{session_id}");
    let payload = json!({"request_id": "fixture-request", "text": "review"});
    let response = http.post(format!("{route}/messages")).json(&payload).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let response = http
        .post(format!("{route}/messages"))
        .bearer_auth(&credential.token)
        .header(USER_ID_HEADER, "untrusted-user")
        .json(&payload)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(response.json::<Value>().await.unwrap()["code"], "busy");

    let foreign = format!("{base}/projects/other-project/cli-workbench/sessions/{session_id}");
    let response = http
        .post(format!("{foreign}/messages"))
        .bearer_auth(&credential.token)
        .json(&payload)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let response =
        http.post(format!("{route}/stop")).bearer_auth(&credential.token).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
