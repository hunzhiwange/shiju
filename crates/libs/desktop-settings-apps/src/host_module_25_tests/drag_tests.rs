use super::page_fixture::{AppsCatalogState, drag_update as update, preferences_route};
use crate::system_settings_apps::drag::*;
use vw_api_app_identity::{
    AppCatalogEntry, AppHostPreferences, AppId, AppInstallation, AppInstallationEnvironment,
    AppInstallationId, AppInstallationRuntimeStatus, AppInstallationSource, AppPreferencesResult,
    AppReadResponse, AppRuntimeKind, AppUserPreferences,
};
fn ids(values: &[&str]) -> Vec<String> {
    values.iter().map(|v| v.to_string()).collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn apps_drag_saves_only_after_drop_and_keeps_last_confirmed_order_on_rejection() {
    use super::page_fixture::preferences;
    use crate::system_settings_apps::{catalog::CatalogRow, presentation};
    use desktop_apps_ui::AgentAppsContext;
    use iced::futures::StreamExt;

    if let Ok(port) = std::env::var("SHIJU_APPS_DRAG_FIXTURE_PORT") {
        let client = vw_gateway_client::GatewayClient::new(gateway_endpoint::GatewayEndpoint::new(
            "127.0.0.1",
            port.parse().unwrap(),
        ))
        .unwrap();
        let mut app = AppsCatalogState {
            host: "fixture-host".into(),
            token: "isolated-fixture-application-token".into(),
            context: Some(AgentAppsContext {
                gateway: client.endpoint().base_url(),
                tenant: client.endpoint().tenant_scope().clone(),
                project: None,
            }),
            ..Default::default()
        };
        for id in [AppId::Claude, AppId::OpenCode, AppId::Codex, AppId::Gemini, AppId::Hermes] {
            app.rows.push(CatalogRow {
                entry: AppCatalogEntry {
                    id,
                    name: format!("{id:?}"),
                    cli_command: Some(presentation::key(id)),
                    runtime_kinds: vec![AppRuntimeKind::NativeCli],
                },
                observation: Ok(AppReadResponse {
                    installations: if id == AppId::OpenCode {
                        vec![]
                    } else {
                        vec![AppInstallation {
                            id: AppInstallationId(presentation::key(id)),
                            app_id: id,
                            target_host_id: "fixture-host".into(),
                            environment: AppInstallationEnvironment::Macos,
                            path: "/fixture/tool".into(),
                            real_path: None,
                            source: AppInstallationSource::System,
                            is_path_default: true,
                            version: Some("1.0".into()),
                            runtime_status: AppInstallationRuntimeStatus::Runnable,
                        }]
                    },
                    desktop: None,
                }),
            });
        }
        let preferences = AppPreferencesResult {
            user: AppUserPreferences {
                app_order: ids(&["claude", "opencode", "codex", "gemini", "hermes"]),
                visible_apps: vec![AppId::Claude, AppId::OpenCode, AppId::Codex, AppId::Gemini],
            },
            host: AppHostPreferences::default(),
            restart_required: false,
            capabilities: None,
        };
        app.preferences.saved = Some(preferences.clone());
        app.preferences.draft = Some(preferences.clone());
        let _ = update(&mut app, DragMessage::Start(AppId::OpenCode));
        assert!(app.drag.source.is_none());
        let _ = update(&mut app, DragMessage::Start(AppId::Claude));
        let _ = update(&mut app, DragMessage::Hover(AppId::Gemini));
        let _ = update(&mut app, DragMessage::Cancel);
        assert_eq!(app.preferences.saved, Some(preferences.clone()));
        assert!(!app.preferences.busy);
        for (source, target, rejected) in
            [(AppId::Claude, AppId::Gemini, false), (AppId::Claude, AppId::Codex, true)]
        {
            let previous = app.preferences.saved.clone();
            let _ = update(&mut app, DragMessage::Start(source));
            let _ = update(&mut app, DragMessage::Hover(target));
            assert_eq!(app.preferences.saved, previous);
            let task = update(&mut app, DragMessage::Drop);
            assert!(app.drag.source.is_none());
            assert!(app.preferences.busy);
            assert_eq!(app.preferences.saved, previous);
            let mut stream = iced_runtime::task::into_stream(task).expect("PATCH task");
            while let Some(action) = stream.next().await {
                if let iced_runtime::Action::Output(message) = action {
                    let _ = preferences_route(&mut app, message);
                }
            }
            assert!(!app.preferences.busy);
            if rejected {
                assert_eq!(app.preferences.saved, previous);
                assert!(app.preferences.error.is_some());
            } else {
                assert_eq!(
                    app.preferences.saved.as_ref().unwrap().user.app_order,
                    ids(&["codex", "opencode", "gemini", "claude", "hermes"])
                );
                assert_eq!(
                    preferences::visible_rows(&app).iter().map(|a| a.entry.id).collect::<Vec<_>>(),
                    vec![AppId::Codex, AppId::OpenCode, AppId::Gemini, AppId::Claude]
                );
            }
        }
        return;
    }
    // The HTTP transport is the external boundary; the child isolates its global endpoint.
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = std::thread::spawn(move || {
        let mut index = 0;
        while index < 2 {
            let (mut stream, _) = listener.accept().unwrap();
            stream.set_read_timeout(Some(std::time::Duration::from_secs(10))).unwrap();
            let mut header = Vec::new();
            while !header.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                stream.read_exact(&mut byte).unwrap();
                header.push(byte[0]);
            }
            let header = String::from_utf8(header).unwrap().to_lowercase();
            if !header.starts_with("patch ") {
                stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}").unwrap();
                continue;
            }
            assert!(
                header.starts_with("patch /v1/agent-apps/preferences?targethostid=fixture-host ")
            );
            assert!(header.contains("x-agent-apps-token: isolated-fixture-application-token"));
            let length: usize = header
                .lines()
                .find_map(|l| l.strip_prefix("content-length:"))
                .unwrap()
                .trim()
                .parse()
                .unwrap();
            let mut body = vec![0; length];
            stream.read_exact(&mut body).unwrap();
            let patch: serde_json::Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(patch["scope"], "user");
            assert_eq!(
                patch["value"]["visibleApps"],
                serde_json::json!(["claude", "opencode", "codex", "gemini"])
            );
            assert_eq!(patch["value"]["appOrder"][1], "opencode");
            assert_eq!(patch["value"]["appOrder"][4], "hermes");
            let (status, body) = if index == 0 {
                (
                    200,
                    serde_json::json!({"user":patch["value"],"host":{},"restartRequired":false})
                        .to_string(),
                )
            } else {
                (403, "{}".to_string())
            };
            write!(stream,"HTTP/1.1 {status} Result\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
            index += 1;
        }
    });
    let status=std::process::Command::new(std::env::current_exe().unwrap()).args(["--exact","host_module_25_tests::drag_tests::apps_drag_saves_only_after_drop_and_keeps_last_confirmed_order_on_rejection","--nocapture"]).env("SHIJU_APPS_DRAG_FIXTURE_PORT",port.to_string()).status().unwrap();
    assert!(status.success());
    server.join().unwrap();
}
