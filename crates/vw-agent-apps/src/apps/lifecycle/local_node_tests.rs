use super::*;
use std::{
    io::{Read, Write},
    os::unix::fs::{PermissionsExt, symlink},
};
fn script(path: &Path, value: &str) {
    std::fs::write(path, value).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
}
#[test]
fn cc_apps_desktop_lifecycle_state_node_gap_rejects_real_host_preview_without_write() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let prefix = root.path().join("prefix");
    let bin = prefix.join("bin");
    let package = prefix.join("lib/node_modules/@openai/codex");
    std::fs::create_dir_all(&bin).unwrap();
    std::fs::create_dir_all(&package).unwrap();
    std::fs::write(package.join("package.json"), r#"{"name":"@openai/codex"}"#).unwrap();
    script(&package.join("cli"), "#!/bin/sh\necho codex 1.0.0\n");
    symlink(package.join("cli"), bin.join("codex")).unwrap();
    script(&bin.join("node"), "#!/bin/sh\necho v18.0.0\n");
    let marker = root.path().join("npm-called");
    script(&bin.join("npm"), &format!("#!/bin/sh\ntouch '{}'\n", marker.display()));
    let env = LocalReadEnvironment {
        host_id: "host".into(),
        home: root.path().into(),
        search_paths: vec![bin.clone()],
        applications: root.path().join("Applications"),
    };
    let write = WriteEnvironment { prefix, npm: bin.join("npm") };
    let report = scoped_discover(AppId::Codex, &env, &write).unwrap();
    let request = AppActionPreviewRequest {
        app_id: AppId::Codex,
        action: AppLifecycleAction::Update,
        target: AppActionTarget::Installation {
            installation_id: report.installations[0].id.clone(),
        },
    };
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let registry = format!("http://{}/", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().unwrap();
            stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
            let mut request = vec![];
            while !request.ends_with(b"\r\n\r\n") {
                let mut b = [0];
                stream.read_exact(&mut b).unwrap();
                request.push(b[0]);
            }
            assert!(String::from_utf8(request).unwrap().contains("codex/latest"));
            let body = r#"{"name":"@openai/codex","version":"2.0.0","engines":{"node":">=22"}}"#;
            write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
        }
    });
    let result = preview_with_node(request.clone(), &env, &write, |inst, w| {
        node_preflight::check_at(inst, w, &registry)
    });
    assert_eq!(result.unwrap_err(), "Node version does not satisfy update requirement");
    assert!(!marker.exists());
    script(&bin.join("node"), "#!/bin/sh\necho v24.0.0\n");
    let plan = preview_with_node(request, &env, &write, |inst, w| {
        node_preflight::check_at(inst, w, &registry)
    })
    .unwrap();
    assert_eq!(plan.source, AppInstallationSource::System);
    assert!(!marker.exists());
    server.join().unwrap();
}
