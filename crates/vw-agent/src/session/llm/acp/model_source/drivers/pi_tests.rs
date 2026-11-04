use super::*;
use crate::{config::schema::AcpAgentConfig, model_relay::RelayIdentity};

#[test]
fn acp_model_source_pi_private_launch_routes_selected_model() {
    let mut profile = AcpLaunchProfile::new(
        &AcpAgentConfig { command: "pi-acp".into(), ..Default::default() },
        RelayIdentity {
            tenant: crate::session::ui_store::current_tenant_scope(),
            project: "project".into(),
            session: "session".into(),
            provider: "provider".into(),
            model: "selected-model".into(),
            generation: 0,
        },
    )
    .unwrap();
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let expected_token = profile.binding.bearer_token().to_owned();
    let server = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut stream = loop {
            if let Ok((stream, _)) = listener.accept() {
                break stream;
            }
            assert!(std::time::Instant::now() < deadline, "child did not make a model request");
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
        let mut request = Vec::new();
        loop {
            let mut chunk = [0; 1024];
            let count = stream.read(&mut chunk).unwrap();
            assert!(count > 0);
            request.extend_from_slice(&chunk[..count]);
            if let Some(split) = request.windows(4).position(|part| part == b"\r\n\r\n") {
                let headers = std::str::from_utf8(&request[..split]).unwrap();
                let length: usize = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse().unwrap())
                    })
                    .unwrap();
                if request.len() >= split + 4 + length {
                    assert!(headers.starts_with("POST /v1/chat/completions HTTP/1.1"));
                    assert!(headers.lines().any(|line| {
                        line.eq_ignore_ascii_case(&format!(
                            "Authorization: Bearer {expected_token}"
                        ))
                    }));
                    let body: serde_json::Value =
                        serde_json::from_slice(&request[split + 4..split + 4 + length]).unwrap();
                    assert_eq!(body["model"], "selected-model");
                    break;
                }
            }
        }
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nBOUND")
            .unwrap();
    });
    configure_native(&mut profile, &endpoint).unwrap();
    let output = std::process::Command::new("python3").args(["-c", "import os,json,pathlib,urllib.request; p=pathlib.Path(os.environ['PI_CODING_AGENT_DIR']); v=json.loads((p/'models.json').read_text())['providers']['session-provider']; assert pathlib.Path(os.environ['PI_ACP_STATE_DIR']).is_dir(); r=urllib.request.Request(v['baseUrl']+'/chat/completions', data=json.dumps({'model':v['models'][0]['id'],'messages':[{'role':'user','content':'hello'}]}).encode(), headers={'Authorization':'Bearer '+os.environ[v['apiKey']], 'Content-Type':'application/json'}); print(urllib.request.urlopen(r).read().decode())"]).envs(&profile.config().env).output().unwrap();
    server.join().unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, b"BOUND\n");
    assert!(profile.config().args.is_empty());
}

fn profile(model: &str) -> AcpLaunchProfile {
    AcpLaunchProfile::new(
        &AcpAgentConfig { command: "pi-acp".into(), ..Default::default() },
        RelayIdentity {
            tenant: crate::session::ui_store::current_tenant_scope(),
            project: "project".into(),
            session: "session".into(),
            provider: "provider".into(),
            model: model.into(),
            generation: 0,
        },
    )
    .unwrap()
}

#[test]
fn acp_model_source_pi_rotates_token_and_model_without_losing_history() {
    let state = tempfile::tempdir().unwrap();
    let mut first = profile("first");
    first.state_root = Some(state.path().to_owned());
    configure_native(&mut first, "http://127.0.0.1:12345").unwrap();
    let native = std::path::PathBuf::from(&first.config().env["PI_CODING_AGENT_DIR"]);
    let acp = std::path::PathBuf::from(&first.config().env["PI_ACP_STATE_DIR"]);
    std::fs::write(acp.join("session-map.json"), b"history").unwrap();
    let binding = first.binding.clone();
    let old_token = binding.bearer_token().to_owned();
    drop(first);
    assert!(!binding.accepts(&old_token, &binding.identity));
    let mut next = profile("second");
    next.state_root = Some(state.path().to_owned());
    configure_native(&mut next, "http://127.0.0.1:12346").unwrap();
    assert_eq!(std::fs::read(acp.join("session-map.json")).unwrap(), b"history");
    assert_ne!(next.config().env["PI_CODING_AGENT_DIR"], native.to_string_lossy());
    let native = std::path::PathBuf::from(&next.config().env["PI_CODING_AGENT_DIR"]);
    let config = std::fs::read_to_string(native.join("models.json")).unwrap();
    assert!(config.contains("second"));
    assert!(!config.contains(&old_token));
    assert!(!config.contains(next.binding.bearer_token()));
    let mut other = profile("second");
    other.binding = crate::model_relay::RelayBinding::new(
        RelayIdentity { session: "other".into(), ..other.binding.identity.clone() },
        std::time::Duration::from_secs(60),
    )
    .unwrap();
    other.state_root = Some(state.path().to_owned());
    configure_native(&mut other, "http://127.0.0.1:12346").unwrap();
    assert_ne!(next.config().env["PI_ACP_STATE_DIR"], other.config().env["PI_ACP_STATE_DIR"]);
}

#[test]
fn acp_model_source_pi_rejects_conflicting_config_and_unpatched_executable() {
    let mut candidate = profile("selected");
    candidate.append_arguments(&["--model".into(), "local".into()]).unwrap();
    assert!(configure_native(&mut candidate, "http://127.0.0.1:12345").is_err());
    for key in [
        "PI_CODING_AGENT_DIR",
        "PI_ACP_STATE_DIR",
        "PI_ACP_PI_COMMAND",
        "VW_PI_RELAY_TOKEN",
        "NODE_OPTIONS",
        "NODE_PATH",
        "PATH",
        "HOME",
        "USERPROFILE",
    ] {
        let config = AcpAgentConfig {
            command: "pi-acp".into(),
            env: HashMap::from([(key.into(), "local".into())]),
            ..Default::default()
        };
        let mut candidate =
            AcpLaunchProfile::new(&config, profile("selected").binding.identity.clone()).unwrap();
        assert!(configure_native(&mut candidate, "http://127.0.0.1:12345").is_err());
        assert_eq!(std::fs::read_dir(candidate.directory()).unwrap().count(), 0);
    }
    let mut candidate = profile("selected");
    assert!(configure_native(&mut candidate, "https://example.invalid").is_err());
    assert!(configure(&mut candidate, "http://127.0.0.1:12345").is_err());
    let executable = candidate.write_config("unpatched", b"#!/bin/sh\nexit 0\n").unwrap();
    let cfg =
        AcpAgentConfig { command: executable.to_string_lossy().into_owned(), ..Default::default() };
    let mut unpatched = AcpLaunchProfile::new(&cfg, candidate.binding.identity.clone()).unwrap();
    assert!(configure(&mut unpatched, "http://127.0.0.1:12345").is_err());
}

#[cfg(unix)]
#[test]
fn acp_model_source_pi_rejects_symlinked_native_configuration() {
    let state = tempfile::tempdir().unwrap();
    let mut first = profile("first");
    first.state_root = Some(state.path().to_owned());
    configure_native(&mut first, "http://127.0.0.1:12345").unwrap();
    let settings: serde_json::Value = serde_json::from_slice(
        &std::fs::read(
            std::path::Path::new(&first.config().env["PI_CODING_AGENT_DIR"]).join("settings.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let path = std::path::PathBuf::from(settings["sessionDir"].as_str().unwrap());
    std::fs::remove_dir(&path).unwrap();
    let sentinel = state.path().join("sentinel");
    std::fs::write(&sentinel, b"unchanged").unwrap();
    std::os::unix::fs::symlink(&sentinel, path).unwrap();
    let mut next = profile("next");
    next.state_root = Some(state.path().to_owned());
    assert!(configure_native(&mut next, "http://127.0.0.1:12346").is_err());
    assert_eq!(std::fs::read(sentinel).unwrap(), b"unchanged");
}

#[tokio::test]
async fn acp_model_source_pi_missing_credentials_and_disabled_model_never_use_native_account() {
    let _lock = crate::test_support::global_async_env_lock().lock().await;
    struct Restore(Option<std::ffi::OsString>);
    impl Drop for Restore {
        fn drop(&mut self) {
            unsafe {
                match self.0.take() {
                    Some(value) => std::env::set_var("VIBEWINDOW_CONFIG_DIR", value),
                    None => std::env::remove_var("VIBEWINDOW_CONFIG_DIR"),
                }
            }
        }
    }
    let root = tempfile::tempdir().unwrap();
    let _restore = Restore(std::env::var_os("VIBEWINDOW_CONFIG_DIR"));
    unsafe {
        std::env::set_var("VIBEWINDOW_CONFIG_DIR", root.path().join("config"));
    }
    let tenant = crate::session::ui_store::current_tenant_scope();
    let (project, _) = crate::project::from_directory(&tenant, root.path()).await.unwrap();
    crate::config::update_project_for_tenant(
        &tenant,
        root.path(),
        serde_json::json!({
            "providers": {"pi-offline-no-account": {
                "api": "http://127.0.0.1:1/v1/chat/completions",
                "adapter":"openai-compatible", "env":[],
                "models": {
                    "active-model":{"status":"active","tool_call":true},
                    "disabled-model":{"status":"disabled","tool_call":true}
                }
            }}
        }),
    )
    .await
    .unwrap();
    let context = crate::project::instance::Context {
        directory: root.path().to_string_lossy().into(),
        worktree: project.worktree.clone(),
        project,
    };
    let relay_context = context.clone();
    crate::project::instance::provide_context(context, async {
        let config = AcpAgentConfig { command:"pi".into(),  ..Default::default() };
        let active = crate::provider::provider::get_model("pi-offline-no-account", "active-model").await.unwrap();
        let mut profile = AcpLaunchProfile::new(&config, RelayIdentity {
            tenant: tenant.clone(), project: relay_context.project.id.clone(),
            session: "missing-key".into(), provider: active.provider_id.clone(),
            model: active.id.clone(), generation: 0,
        }).unwrap();
        let server = crate::model_relay::RelayServer::start(
            profile.binding.clone(), std::sync::Arc::new(crate::model_relay::ProviderRelayBackend::new().unwrap()), relay_context,
        ).await.unwrap();
        let endpoint = format!("http://{}/v1", server.address());
        profile.attach_server(server);
        configure_native(&mut profile, endpoint.trim_end_matches("/v1")).unwrap();
        let response = crate::model_relay::upstream_http_client().unwrap()
            .post(format!("{endpoint}/chat/completions"))
            .bearer_auth(profile.binding.bearer_token())
            .json(&serde_json::json!({"model":"active-model","messages":[{"role":"user","content":"hello"}]}))
            .send().await.unwrap();
        assert_eq!(response.status(), 401);
        // A stale caller snapshot cannot resurrect a catalog-disabled model.
        let mut disabled = active.clone();
        disabled.id = "disabled-model".into();
        let result = crate::session::llm::acp::model_source::prepare("pi", &config, &disabled, "disabled").await;
        let error = match result {
            Ok(_) => panic!("disabled model must not create a Pi launch"),
            Err(error) => error.to_string(),
        };
        assert!(error.contains("disabled-model"));
    }).await;
}
