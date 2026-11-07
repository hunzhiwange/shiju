use super::env::restore;
use crate::app::state::{
    AgentAppsContext,
    app_providers::{
        env::{Form, cleanup_selection, restore_request},
        list::Scope,
    },
};
use std::io::{BufRead, BufReader, Read, Write};
use vw_gateway_client::{
    GatewayClient, GatewayEndpoint,
    vw_api_types::{
        agent_apps::AppId,
        app_providers::{
            env_clean::EnvCleanSelection,
            env_diagnose::{EnvDiagnostic, EnvSource, SystemShellFile, UserShellFile},
            env_restore::EnvRestoreStatus,
        },
    },
};

#[test]
fn cc04_ui_env_readonly_source_has_no_delete() {
    let receipt =
        EnvCleanSelection { diagnostic_id: "test-diagnostic".into(), base_digest: "base".into() };
    let mut entry = EnvDiagnostic {
        variable: "ANTHROPIC_API_KEY".into(),
        source: EnvSource::UserShell { file: UserShellFile::Zshrc, line: 4 },
        cleanup_available: true,
        cleanup: Some(receipt.clone()),
    };
    assert_eq!(cleanup_selection(&entry), Some(&receipt));
    let writable = entry.clone();
    entry.cleanup_available = false;
    assert!(cleanup_selection(&entry).is_none(), "只读用户文件不应提供清理");
    entry.cleanup_available = true;
    entry.source = EnvSource::SystemShell { file: SystemShellFile::Profile, line: 4 };
    assert!(cleanup_selection(&entry).is_none(), "系统文件不应提供清理");
    let system = entry.clone();
    entry.source = EnvSource::ProcessInherited;
    assert!(cleanup_selection(&entry).is_none(), "继承环境不应提供清理");
    let inherited = entry.clone();
    entry.source = EnvSource::UserShell { file: UserShellFile::Zshrc, line: 4 };
    entry.cleanup = None;
    assert!(cleanup_selection(&entry).is_none(), "没有执行端回执时不应提供清理");
    let mut form = Form { entries: vec![writable, system, inherited, entry], ..Form::default() };
    for index in 1..4 {
        form.toggle(index, true);
    }
    assert!(form.clean_request("fixture-host").is_none());
    form.toggle(0, true);
    assert_eq!(form.clean_request("fixture-host").unwrap().selections, vec![receipt]);
}

#[tokio::test]
async fn cc04_ui_env_restore_uses_backup_id() {
    let backup_id = "ab".repeat(32);
    let mut form = Form::default();
    form.record_backup(backup_id.clone());
    let request =
        restore_request("fixture-host", &form.backup_id).expect("应使用清理返回的备份编号");
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = GatewayEndpoint::new("127.0.0.1", listener.local_addr().unwrap().port());
    let scope = Scope {
        context: AgentAppsContext {
            gateway: endpoint.base_url(),
            tenant: endpoint.tenant_scope().clone(),
            project: None,
        },
        host: "fixture-host".into(),
        app: AppId::Claude,
    };
    let server = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(std::time::Instant::now() < deadline, "SDK 未连接");
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(e) => panic!("{e}"),
            }
        };
        stream.set_read_timeout(Some(std::time::Duration::from_secs(3))).unwrap();
        let mut reader = BufReader::new(&mut stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        assert!(line.starts_with("POST /v1/app-providers/claude/environment/restore "));
        let mut len = 0;
        let mut authorized = false;
        loop {
            line.clear();
            assert_ne!(reader.read_line(&mut line).unwrap(), 0);
            if line == "\r\n" {
                break;
            }
            let line = line.trim().to_ascii_lowercase();
            if let Some(value) = line.strip_prefix("content-length:") {
                len = value.trim().parse().unwrap();
            }
            authorized |= line == "x-agent-apps-token: fake-host-token";
        }
        assert!(authorized);
        let mut bytes = vec![0; len];
        reader.read_exact(&mut bytes).unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let response = r#"{"items":[{"file":"zshrc","status":"restored"}]}"#;
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}",
            response.len()
        )
        .unwrap();
        body
    });
    let client = GatewayClient::new(endpoint).unwrap();
    let result = restore(client, scope, request, "fake-host-token".into()).await.unwrap();
    assert_eq!(result.items[0].status, EnvRestoreStatus::Restored);
    assert_eq!(
        server.join().unwrap(),
        serde_json::json!({"targetHostId": "fixture-host", "backupId": backup_id})
    );
}
