use super::catalog::*;
use vw_gateway_client::{AgentAppsReadError, vw_api_types::agent_apps::*};

fn row() -> CatalogRow {
    CatalogRow {
        entry: AppCatalogEntry {
            id: AppId::Codex,
            name: "中文应用".into(),
            cli_command: Some("codex".into()),
            runtime_kinds: vec![AppRuntimeKind::NativeCli, AppRuntimeKind::AcpAdapter],
        },
        observation: Ok(AppReadResponse { installations: vec![], desktop: None }),
    }
}

#[test]
fn cc_apps_desktop_catalog_state_empty_and_independent_runtimes() {
    let item = row();
    assert_eq!(runtime_status(&item, AppRuntimeKind::NativeCli), "未安装");
    assert_eq!(runtime_status(&item, AppRuntimeKind::AcpAdapter), "未验证（请在 ACP 页管理）");
}

#[test]
fn cc_apps_desktop_catalog_state_explicit_failures_and_desktop_presence() {
    let mut item = row();
    item.observation = Err(AgentAppsReadError::Http { status: 503, body: serde_json::json!({}) });
    assert!(runtime_status(&item, AppRuntimeKind::NativeCli).contains("远端离线"));
    item.observation = Err(AgentAppsReadError::Http { status: 501, body: serde_json::json!({}) });
    assert_eq!(runtime_status(&item, AppRuntimeKind::NativeCli), "不支持");
    item.observation = Ok(AppReadResponse {
        installations: vec![],
        desktop: Some(AppDesktopObservation {
            supported: true,
            app_bundles: Some(vec!["/应用/Claude.app".into()]),
            config_directories: vec![],
        }),
    });
    assert_eq!(runtime_status(&item, AppRuntimeKind::DesktopApp), "已发现，运行未验证");
}

#[test]
fn cc_apps_desktop_catalog_state_retry_and_stale_response() {
    use crate::app::state::{AgentAppsContext, AppsCatalogState};
    let context = AgentAppsContext {
        gateway: "http://gateway".into(),
        tenant: vw_gateway_client::vw_api_types::id::TenantScope::new("p", "c").unwrap(),
        project: None,
    };
    let mut state = AppsCatalogState::default();
    let first = state.begin(context.clone());
    assert!(state.finish(&context, first, Err("远端离线".into())));
    let second = state.begin(context.clone());
    assert!(state.loading && state.error.is_none());
    assert!(!state.finish(&context, first, Ok(vec![row()])));
    assert!(state.finish(&context, second, Ok(vec![row()])));
    assert_eq!(state.rows.len(), 1);
    assert!(
        !format!(
            "{:?}",
            AppsCatalogState { token: "secret-never-log".into(), ..Default::default() }
        )
        .contains("secret")
    );
}

#[test]
fn cc_apps_desktop_catalog_state_damaged_runnable_unverified() {
    let mut item = row();
    let mut installation = AppInstallation {
        id: AppInstallationId("opaque".into()),
        app_id: AppId::Codex,
        target_host_id: "remote".into(),
        environment: AppInstallationEnvironment::Wsl { distribution: "中文发行版".into() },
        path: "/远端/codex".into(),
        real_path: None,
        source: AppInstallationSource::Unknown,
        is_path_default: true,
        version: None,
        runtime_status: AppInstallationRuntimeStatus::ProbeFailed {
            reason: AppInstallationProbeFailure::DamagedInstallation,
        },
    };
    for (status, expected) in [
        (installation.runtime_status.clone(), "损坏"),
        (AppInstallationRuntimeStatus::Runnable, "可运行"),
        (
            AppInstallationRuntimeStatus::ProbeFailed {
                reason: AppInstallationProbeFailure::InvalidVersionOutput,
            },
            "探测失败，未验证",
        ),
        (AppInstallationRuntimeStatus::ExitedWithError { exit_code: 2 }, "运行失败"),
    ] {
        installation.runtime_status = status;
        item.observation =
            Ok(AppReadResponse { installations: vec![installation.clone()], desktop: None });
        assert_eq!(runtime_status(&item, AppRuntimeKind::NativeCli), expected);
        assert_eq!(runtime_status(&item, AppRuntimeKind::AcpAdapter), "未验证（请在 ACP 页管理）");
    }
}

#[tokio::test]
async fn cc_apps_desktop_catalog_state_sdk_fourteen_concurrent_ordered_and_partial_failure() {
    use std::io::{Read, Write};
    use vw_gateway_client::{GatewayClient, GatewayEndpoint};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let ids = [
        "claude",
        "claude-desktop",
        "codex",
        "gemini",
        "grokbuild",
        "opencode",
        "openclaw",
        "hermes",
        "pi",
        "mcode",
        "kimi",
        "dsh",
        "agy",
        "omp",
    ];
    let server = std::thread::spawn(move || {
        let apps = ids.iter().map(|id| serde_json::json!({"id":id,"name":format!("中文应用 {id}"),"cliCommand":null,"runtimeKinds":[if *id == "claude-desktop" {"desktop_app"} else {"native_cli"}]})).collect::<Vec<_>>();
        listener.set_nonblocking(true).unwrap();
        let mut pending = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for index in 0..15 {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(
                            std::time::Instant::now() < deadline,
                            "expected concurrent requests"
                        );
                        std::thread::sleep(std::time::Duration::from_millis(5));
                    }
                    Err(error) => panic!("accept failed: {error}"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
            let mut request = Vec::new();
            while !request.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                stream.read_exact(&mut byte).unwrap();
                request.push(byte[0]);
            }
            let request = String::from_utf8(request).unwrap();
            assert!(request.contains("x-agent-apps-token: fixture-application-grant"));
            assert!(request.contains("targetHostId=remote"));
            if index > 0 {
                let id = ids
                    .iter()
                    .find(|id| {
                        request.contains(&format!("appId={id} "))
                            || request.contains(&format!("appId={id}&"))
                    })
                    .unwrap();
                assert!(seen.insert(*id));
            }
            let (status, body) = if index == 0 {
                (200, serde_json::json!({"apps": apps}).to_string())
            } else if request.contains("appId=codex") {
                (503, "{}".into())
            } else {
                (200, r#"{"installations":[]}"#.into())
            };
            pending.push((stream, status, body));
            // Hold the first wave until all six requests arrive, then reply backwards.
            if (1..6).contains(&index) {
                continue;
            }
            if index == 6 {
                assert!(
                    matches!(listener.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock)
                );
            }
            for (mut stream, status, body) in pending.drain(..).rev() {
                write!(stream, "HTTP/1.1 {status} Result\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            }
        }
        assert_eq!(seen.len(), 14);
    });
    let rows = load(
        GatewayClient::new(GatewayEndpoint::new("127.0.0.1", port)).unwrap(),
        "remote".into(),
        "fixture-application-grant".into(),
    )
    .await
    .unwrap();
    server.join().unwrap();
    assert_eq!(rows.len(), 14);
    assert_eq!(rows[2].entry.id, AppId::Codex);
    assert!(runtime_status(&rows[2], AppRuntimeKind::NativeCli).contains("远端离线"));
    assert_eq!(runtime_status(&rows[3], AppRuntimeKind::NativeCli), "未安装");
}

#[test]
fn cc_apps_desktop_catalog_state_context_switch_releases_pending_load() {
    use crate::app::state::{AgentAppsContext, AppsCatalogState};
    let old = AgentAppsContext {
        gateway: "old".into(),
        tenant: vw_gateway_client::vw_api_types::id::TenantScope::new("p", "c").unwrap(),
        project: None,
    };
    let current = AgentAppsContext { gateway: "new".into(), ..old.clone() };
    let mut state = AppsCatalogState { token: "do-not-forward".into(), ..Default::default() };
    let generation = state.begin(old.clone());
    assert!(state.reset_if_context_changed(&current));
    assert!(!state.loading);
    assert!(state.token.is_empty() && state.rows.is_empty() && !state.connection_open);
    assert!(!state.finish(&old, generation, Ok(vec![row()])));
    let retry = state.begin(current.clone());
    assert!(state.finish(&current, retry, Ok(vec![row()])));
    assert_eq!(state.rows.len(), 1);
}

#[test]
fn local_apps_credentials_only_apply_to_current_connection_and_generation() {
    use crate::app::{
        App,
        message::{SettingsMessage, settings::AppsCatalogMessage},
        state::AgentAppsContext,
    };
    let mut app = App::new().0;
    let client = crate::app::config::gateway_client().unwrap();
    let context = AgentAppsContext {
        gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(),
        project: app.project_path.clone(),
    };
    let generation = app.apps_catalog.begin(context.clone());
    let mut other = context.clone();
    other.gateway = "http://other-gateway".into();
    for (context, generation) in
        [(other, generation), (context.clone(), generation.wrapping_sub(1))]
    {
        let _ = app.update(crate::app::Message::Settings(SettingsMessage::AppsCatalog(
            AppsCatalogMessage::LocalReady {
                context,
                generation,
                result: Ok(vw_shared::local_apps::Credentials {
                    token: "stale-local-credential".into(),
                }),
            },
        )));
        assert!(app.apps_catalog.token.is_empty());
    }
    let _ = app.update(crate::app::Message::Settings(SettingsMessage::AppsCatalog(
        AppsCatalogMessage::LocalReady {
            context,
            generation,
            result: Ok(vw_shared::local_apps::Credentials {
                token: "current-local-credential".into(),
            }),
        },
    )));
    assert_eq!(app.apps_catalog.host, vw_shared::local_apps::HOST_ID);
    assert_eq!(app.apps_catalog.token, "current-local-credential");
    assert!(app.apps_catalog.loading);
    assert!(!app.apps_catalog.connection_open);
}

#[test]
fn catalog_cache_expires_and_invalidates_after_failure_or_context_change() {
    use crate::app::state::{AgentAppsContext, AppsCatalogState};
    let context = AgentAppsContext {
        gateway: "http://gateway".into(),
        tenant: vw_gateway_client::vw_api_types::id::TenantScope::new("p", "c").unwrap(),
        project: None,
    };
    let mut state =
        AppsCatalogState { host: "host".into(), token: "fixture".into(), ..Default::default() };
    let generation = state.begin(context.clone());
    assert!(!state.cache_is_fresh());
    assert!(state.finish(&context, generation, Ok(vec![row()])));
    assert!(state.cache_is_fresh());
    state.loaded_at = Some(web_time::Instant::now() - std::time::Duration::from_secs(31));
    assert!(!state.cache_is_fresh());
    let generation = state.begin(context.clone());
    let mut failed = row();
    failed.observation = Err(AgentAppsReadError::Http { status: 503, body: serde_json::json!({}) });
    assert!(state.finish(&context, generation, Ok(vec![failed])));
    assert!(!state.cache_is_fresh());
    let generation = state.begin(context.clone());
    assert!(state.finish(&context, generation, Ok(vec![])));
    assert!(state.cache_is_fresh());
    let other = AgentAppsContext { gateway: "http://other".into(), ..context };
    assert!(state.reset_if_context_changed(&other));
    assert!(!state.cache_is_fresh());
}

#[test]
fn catalog_reopening_reuses_cache_but_manual_refresh_and_credentials_invalidate_it() {
    use crate::app::{
        App,
        message::{SettingsMessage, settings::AppsCatalogMessage},
        state::AgentAppsContext,
    };
    let mut app = App::new().0;
    let client = crate::app::config::gateway_client().unwrap();
    let context = AgentAppsContext {
        gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(),
        project: app.project_path.clone(),
    };
    app.apps_catalog.host = "remote".into();
    app.apps_catalog.token = "fixture".into();
    let generation = app.apps_catalog.begin(context.clone());
    assert!(app.apps_catalog.finish(&context, generation, Ok(vec![row()])));
    let _ = app.update(crate::app::Message::Settings(SettingsMessage::SystemTabSelected(
        crate::app::components::system_settings::SystemTab::Apps,
    )));
    assert!(!app.apps_catalog.loading);
    assert_eq!(app.apps_catalog.generation, generation);
    assert_eq!(app.apps_catalog.rows.len(), 1);
    let _ = app.update(crate::app::Message::Settings(SettingsMessage::AppsCatalog(
        AppsCatalogMessage::Refresh,
    )));
    assert!(app.apps_catalog.loading);
    assert!(!app.apps_catalog.cache_is_fresh());
    let generation = app.apps_catalog.generation;
    assert!(app.apps_catalog.finish(&context, generation, Ok(vec![row()])));
    let _ = app.update(crate::app::Message::Settings(SettingsMessage::AppsCatalog(
        AppsCatalogMessage::TokenInput("replacement".into()),
    )));
    assert!(!app.apps_catalog.cache_is_fresh());
    assert!(app.apps_catalog.rows.is_empty());
    let generation = app.apps_catalog.begin(context.clone());
    assert!(app.apps_catalog.finish(&context, generation, Ok(vec![row()])));
    let _ = app.update(crate::app::Message::Settings(SettingsMessage::AppsCatalog(
        AppsCatalogMessage::HostInput("other".into()),
    )));
    assert!(!app.apps_catalog.cache_is_fresh());
    assert!(app.apps_catalog.rows.is_empty());
}
