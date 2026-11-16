use gateway_app_diagnostics::GatewayAppDiagnosticsClient;
use crate::{app_providers::AppProviderStore, global_database::GlobalDatabase};
use serde_json::{Value, json};
use std::{os::unix::fs::PermissionsExt, time::Duration};
use tokio::{io::{AsyncReadExt}, net::TcpListener};
use vw_api_app_identity::AppId;
use vw_api_app_diagnostics::*;
use vw_api_app_provider_base::{AppProviderWrite, ProviderProtocol};
use vw_api_transport::id::TenantScope;

/// Also exercise bootstrap-owned extensions in the real standalone gateway process.
#[tokio::test]
async fn diagnostic_macos_jobs_acceptance_gateway() -> anyhow::Result<()> {
    use super::app_routing::lifecycle_api_tests::{
        HOST, MANAGER, client, spawn_gateway, write_policy,
    };
    let root = tempfile::tempdir()?;
    let credentials = root.path().join("credentials");
    std::fs::create_dir(&credentials)?;
    std::fs::set_permissions(&credentials, std::fs::Permissions::from_mode(0o700))?;
    let policy = root.path().join("access.json");
    write_policy(&policy, root.path(), 0);
    let mut access: Value = serde_json::from_slice(&std::fs::read(&policy)?)?;
    access["principals"][0]["grants"][0]["provider_config_read"] = json!(true);
    access["principals"][0]["grants"][0]["routing_apps"] =
        json!([{"project_id":"project","app_id":"claude"}]);
    std::fs::write(&policy, access.to_string())?;
    let upstream = TcpListener::bind("127.0.0.1:0").await?;
    AppProviderStore::new(GlobalDatabase::for_home(root.path())?)
        .map_err(anyhow::Error::msg)?
        .save(
            &TenantScope::new("p", "owner")?,
            AppId::Claude,
            "provider",
            &AppProviderWrite {
                name: "Controlled standalone".into(),
                protocol: ProviderProtocol::Anthropic,
                api_key: "fixture-key".into(),
                base_url: format!("http://{}/v1", upstream.local_addr()?),
                model: "fixture-model".into(),
                request_options: None,
            },
        )
        .map_err(anyhow::Error::msg)?;
    let (gateway, port) = spawn_gateway(root.path(), &policy).await;
    let sdk = client(port, "owner");
    let scope = DiagnosticScopeQuery {
        project_id: "project".into(),
        target_host_id: HOST.into(),
        app_id: AppId::Claude,
    };
    let job = GatewayAppDiagnosticsClient::new(sdk.transport().clone()).diagnostic_start(
            &scope,
            &DiagnosticStartRequest::Reachability { provider_id: "provider".into() },
            MANAGER,
        )
        .await?;
    let (mut socket, _) = tokio::time::timeout(Duration::from_secs(5), upstream.accept()).await??;
    let mut headers = Vec::new();
    tokio::time::timeout(Duration::from_secs(5), async {
        while !headers.ends_with(b"\r\n\r\n") {
            headers.push(socket.read_u8().await.unwrap());
        }
    })
    .await?;
    assert!(headers.starts_with(b"GET /v1 HTTP/1.1"));
    assert_eq!(
        GatewayAppDiagnosticsClient::new(sdk.transport().clone()).diagnostic_status(&scope, &job.task_id, MANAGER).await?.state,
        DiagnosticJobState::Running {}
    );
    GatewayAppDiagnosticsClient::new(sdk.transport().clone()).diagnostic_cancel(&scope, &job.task_id, MANAGER).await?;
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let done = GatewayAppDiagnosticsClient::new(sdk.transport().clone()).diagnostic_status(&scope, &job.task_id, MANAGER).await.unwrap();
            if done.state == (DiagnosticJobState::Cancelled {}) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await?;
    assert_eq!(tokio::time::timeout(Duration::from_secs(5), socket.read(&mut [0])).await??, 0);
    drop(gateway); // GatewayProcess::drop kills and waits for the standalone child.
    eprintln!(
        "standalone gateway: SDK POST=202, GET running=200, cancel=200, cancelled=200, actual TCP EOF, gateway reaped"
    );
    Ok(())
}
