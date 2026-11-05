use super::*;
#[test]
fn cc_apps_node_requirement_gap_ranges() {
    for (range, version, expected) in [
        (">=24.16.0 <25 || >=26.1.0", "22.23.2", Some(false)),
        (">=24.16.0 <25 || >=26.1.0", "24.16.0", Some(true)),
        ("^20.19.0 || >=22.12.0", "21.0.0", Some(false)),
        ("18 - 20", "20.9.0", Some(true)),
        ("~18.17", "18.18.0", Some(false)),
        ("18.x", "18.20.8", Some(true)),
        ("^0.2.3", "0.3.0", Some(false)),
        ("node >=20", "18.0.0", None),
        ("lts/* || >=18", "20.0.0", Some(true)),
    ] {
        assert_eq!(node_range_satisfied(range, version), expected, "{range}");
    }
}

fn installation() -> vw_api_types::agent_apps::AppInstallation {
    use vw_api_types::agent_apps::*;
    AppInstallation {
        id: AppInstallationId("selected".into()),
        app_id: AppId::OpenClaw,
        target_host_id: "host".into(),
        environment: AppInstallationEnvironment::Macos,
        path: "/selected/bin/openclaw".into(),
        real_path: Some("/selected/lib/node_modules/openclaw/bin.js".into()),
        source: AppInstallationSource::Nvm,
        is_path_default: false,
        version: None,
        runtime_status: AppInstallationRuntimeStatus::Runnable,
    }
}

#[test]
fn cc_apps_node_requirement_gap_instance_and_environment() {
    let mut inst = installation();
    assert_eq!(
        node_candidate(&inst, Some("/path/node"), |p| p == "/selected/bin/node"),
        Some("/selected/bin/node".into())
    );
    assert_eq!(node_candidate(&inst, Some("/path/node"), |_| false), Some("/path/node".into()));
    assert!(applicable(&inst));
    inst.environment = Environment::Windows;
    inst.path = "C:\\selected\\openclaw.cmd".into();
    assert_eq!(
        node_candidate(&inst, None, |p| p == "C:/selected/node.exe"),
        Some("C:/selected/node.exe".into())
    );
    assert!(applicable(&inst));
    inst.environment = Environment::Wsl { distribution: "Ubuntu".into() };
    assert!(!applicable(&inst));
    inst.environment = Environment::Macos;
    for source in [Source::Bun, Source::Homebrew, Source::Native] {
        inst.source = source;
        assert!(!applicable(&inst));
    }
    inst.source = Source::Nvm;
    inst.app_id = AppId::Mcode;
    assert!(!applicable(&inst));
    inst.app_id = AppId::OpenClaw;
    inst.real_path = Some("/native/openclaw".into());
    assert!(!applicable(&inst));
}

#[cfg(unix)]
#[test]
fn cc_apps_node_requirement_gap_real_sibling_reprobe() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("node");
    let mut inst = installation();
    inst.path = dir.path().join("openclaw").to_str().unwrap().into();
    for version in ["24.16.0", "22.0.0"] {
        std::fs::write(&path, format!("#!/bin/sh\nprintf 'v{version}\\n'\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        let candidate = node_candidate(&inst, None, |p| std::path::Path::new(p).is_file()).unwrap();
        let probe = crate::apps::discovery::version::probe_local_version(
            std::path::Path::new(&candidate),
            Duration::from_secs(1),
        )
        .unwrap();
        let result = evaluate_requirement(
            &serde_json::json!({"engines":{"node":">=24.16.0"}}),
            probe.version.as_deref().unwrap(),
        );
        assert_eq!(enforce(result).is_err(), version == "22.0.0");
    }
}

#[tokio::test]
async fn cc_apps_node_requirement_gap_http_and_unknown() {
    use std::io::{Read, Write};
    for (status, body, expected) in [
        (
            "200 OK",
            r#"{"engines":{"node":">=24.16.0"}}"#,
            NodeRequirement::Gap { required: ">=24.16.0".into(), current: "22.0.0".into() },
        ),
        (
            "200 OK",
            r#"{"engines":{"node":">=20"}}"#,
            NodeRequirement::Satisfied { required: ">=20".into(), current: "22.0.0".into() },
        ),
        ("200 OK", r#"{"engines":{}}"#, NodeRequirement::Unknown),
        ("503 Unavailable", "{}", NodeRequirement::Unknown),
        ("200 OK", "broken", NodeRequirement::Unknown),
    ] {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/package/1.0.0", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 4096];
            let count = stream.read(&mut request).unwrap();
            assert!(String::from_utf8_lossy(&request[..count]).starts_with("GET /package/1.0.0 "));
            write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        });
        let result = fetch_requirement(&reqwest::Client::new(), &url, "22.0.0").await;
        assert_eq!(result, expected);
        let rejected = matches!(result, NodeRequirement::Gap { .. });
        assert_eq!(enforce(result).is_err(), rejected);
        server.join().unwrap();
    }
    let inst = installation();
    let result = ensure_update_node(&reqwest::Client::new(), &inst, "1.0.0", |selected| {
        assert_eq!(selected.id, inst.id);
        None
    })
    .await;
    assert_eq!(result, Ok(NodeRequirement::Unknown));
}
