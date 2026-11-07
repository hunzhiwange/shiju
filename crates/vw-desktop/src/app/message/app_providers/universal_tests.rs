use super::universal::{Outcome, finish, project};
use crate::app::state::{
    AgentAppsContext,
    app_providers::{list::Scope, universal::Form},
};
use std::io::{BufRead, BufReader, Read, Write};
use vw_gateway_client::{
    GatewayClient, GatewayEndpoint,
    vw_api_types::{
        agent_apps::AppId,
        app_providers::universal_sync::{
            UniversalCatalogResult, UniversalLiveResult, UniversalSyncError, UniversalSyncResult,
        },
    },
};

fn scope(client: &GatewayClient) -> Scope {
    Scope {
        context: AgentAppsContext {
            gateway: client.endpoint().base_url(),
            tenant: client.endpoint().tenant_scope().clone(),
            project: None,
        },
        host: "fixture-host".into(),
        app: AppId::Claude,
    }
}

fn draft() -> Form {
    Form {
        selected: Some("fixture".into()),
        apps: vec![AppId::Claude, AppId::Codex],
        busy: true,
        ..Form::default()
    }
}

#[tokio::test]
async fn cc04_ui_universal_validation_failure_preserves_draft() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let client = GatewayClient::new(GatewayEndpoint::new(
        "127.0.0.1",
        listener.local_addr().unwrap().port(),
    ))
    .unwrap();
    let scope = scope(&client);
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
        let mut reader = BufReader::new(&mut stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        assert_eq!(line, "POST /v1/universal-providers/fixture/sync HTTP/1.1\r\n");
        let mut len = 0;
        loop {
            line.clear();
            assert_ne!(reader.read_line(&mut line).unwrap(), 0);
            if line == "\r\n" {
                break;
            }
            if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                len = value.trim().parse().unwrap();
            }
        }
        let mut bytes = vec![0; len];
        reader.read_exact(&mut bytes).unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body, serde_json::json!({"apps": ["claude", "codex"]}));
        write!(
            stream,
            "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
    });
    let mut form = draft();
    let result = project(client, form.selected.clone().unwrap(), form.apps.clone()).await;
    server.join().unwrap();
    assert!(finish(&mut form, (&scope, 3), (&scope, 3), result.map(Outcome::Projected)));
    assert_eq!(form.selected.as_deref(), Some("fixture"));
    assert_eq!(form.apps, [AppId::Claude, AppId::Codex]);
    assert!(form.error.as_deref().unwrap().contains("400"));
    assert!(form.results.is_empty());
    assert!(!form.busy);
}

#[test]
fn cc04_ui_universal_stale_response_is_ignored() {
    let client = GatewayClient::new(GatewayEndpoint::new("127.0.0.1", 1)).unwrap();
    let scope = scope(&client);
    let mut other = scope.clone();
    other.context.project = Some("/other-project".into());
    let mut form = draft();
    let results = vec![
        UniversalSyncResult {
            app_id: AppId::Claude,
            provider_id: Some("universal-claude-fixture".into()),
            catalog: UniversalCatalogResult::Updated,
            live: UniversalLiveResult::NotAttempted,
        },
        UniversalSyncResult {
            app_id: AppId::Codex,
            provider_id: None,
            catalog: UniversalCatalogResult::Failed { code: UniversalSyncError::AppDisabled },
            live: UniversalLiveResult::NotAttempted,
        },
    ];
    for completed in [(&scope, 3), (&other, 4)] {
        assert!(!finish(
            &mut form,
            (&scope, 4),
            completed,
            Ok(Outcome::Projected(results.clone()))
        ));
        assert!(form.busy);
        assert!(form.results.is_empty());
        assert!(form.error.is_none());
    }
    assert!(finish(&mut form, (&scope, 4), (&scope, 4), Ok(Outcome::Projected(results.clone()))));
    assert!(!form.busy);
    assert_eq!(form.results, results);
    assert!(form.refresh_catalog_on_close);
    assert_eq!(form.apps, [AppId::Claude, AppId::Codex]);
}
