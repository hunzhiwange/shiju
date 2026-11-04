//! Protocol assertions use a controlled HTTP service; disk acceptance lives in gateway local_import tests.
use crate::app::message::{ProjectMessage as P, ViewMessage};
use crate::app::{App, Message};
use iced::futures::StreamExt;
use serde_json::json;
use std::io::{Read, Write};

async fn messages(task: iced::Task<Message>) -> Vec<Message> {
    let Some(mut stream) = iced_runtime::task::into_stream(task.collect()) else { return vec![] };
    while let Some(action) = stream.next().await {
        if let iced_runtime::Action::Output(messages) = action {
            return messages;
        }
    }
    vec![]
}

fn isolated(name: &str) -> bool {
    if std::env::var("SHIJU_EXTERNAL_DROP_CHILD").as_deref() == Ok(name) {
        return false;
    }
    let home = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--test-threads=1"])
        .env("SHIJU_EXTERNAL_DROP_CHILD", name)
        .env("HOME", home.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    true
}

fn configure(host: &str, port: u16) {
    let mut config = vw_config_types::ui::GatewayClientSystemSettingsConfig::default();
    config.set_servers(
        vec![vw_config_types::ui::GatewayClientServerConfig {
            id: "external-test".into(),
            host: host.into(),
            port,
            ..Default::default()
        }],
        "external-test".into(),
    );
    let path = vw_config_types::paths::home_config_dir(std::env::var_os("HOME").unwrap())
        .join("gateway-client-bootstrap.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, serde_json::to_vec(&config).unwrap()).unwrap();
}

#[tokio::test]
async fn file_tree_external_drop_remote_rejected_before_project_resolution() {
    if isolated(concat!(
        module_path!(),
        "::file_tree_external_drop_remote_rejected_before_project_resolution"
    )) {
        return;
    }
    configure("192.0.2.1", 9);
    let error = super::super::helpers::gateway_import_local_paths(
        "/project",
        "/project",
        vec!["/source".into()],
    )
    .await
    .unwrap_err();
    assert!(error.contains("不支持远程"), "{error}");
}

#[tokio::test]
async fn file_tree_external_drop_tasks_keep_each_source_and_project_after_switch_and_partial_failure()
 {
    if isolated(concat!(
        module_path!(),
        "::file_tree_external_drop_tasks_keep_each_source_and_project_after_switch_and_partial_failure"
    )) {
        return;
    }
    let root_dir = tempfile::tempdir().unwrap();
    let source_dir = tempfile::tempdir().unwrap();
    let root = root_dir.path().to_string_lossy().into_owned();
    let source = source_dir.path().join("中文.bin").to_string_lossy().into_owned();
    std::fs::write(&source, [0, 255, 9]).unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    configure("127.0.0.1", listener.local_addr().unwrap().port());
    let server_root = root.clone();
    let expected_source = source.clone();
    let server = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        let mut imports = vec![];
        while imports.len() < 3 && std::time::Instant::now() < deadline {
            let (mut stream, _) = match listener.accept() {
                Ok(value) => value,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                    continue;
                }
                Err(e) => panic!("{e}"),
            };
            stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
            let mut bytes = vec![];
            let mut buf = [0; 4096];
            let end = loop {
                let count = stream.read(&mut buf).unwrap();
                assert_ne!(count, 0);
                bytes.extend_from_slice(&buf[..count]);
                if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&bytes[..end]).to_ascii_lowercase();
                    let length: usize = headers
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length:"))
                        .unwrap_or("0")
                        .trim()
                        .parse()
                        .unwrap();
                    if bytes.len() >= end + 4 + length {
                        break end;
                    }
                }
            };
            let headers = String::from_utf8_lossy(&bytes[..end]);
            let request: serde_json::Value =
                serde_json::from_slice(&bytes[end + 4..]).unwrap_or(serde_json::Value::Null);
            let response = if headers.starts_with("POST /v1/projects/resolve ") {
                assert!(
                    request["directory"] == server_root
                        || request["directory"] == format!("{server_root}/branch")
                );
                json!({"project":{"id":"project-test","name":"test","directory":server_root,"display_path":server_root,"status":"ready","created_at_ms":0,"updated_at_ms":0,"git":{"is_repo":false,"has_uncommitted_changes":false}}})
            } else if headers.starts_with("GET ") {
                assert!(headers.contains("/worktrees "));
                json!({"items":[{"id":"worktree-test","project_id":"project-test","name":"branch","branch":"branch","directory":format!("{server_root}/branch"),"status":"ready","created_at_ms":0,"updated_at_ms":0}]})
            } else {
                assert!(headers.starts_with("POST /v1/files/import-local "), "{headers}");
                assert_eq!(request["project_id"], "project-test");
                if imports.len() < 2 {
                    assert!(request.get("worktree_id").is_none());
                } else {
                    assert_eq!(request["worktree_id"], "worktree-test");
                }
                assert_eq!(request["source_paths"], json!([expected_source]));
                assert_eq!(
                    request["target_directory"],
                    if imports.is_empty() { "dst" } else { "" }
                );
                let ok = imports.is_empty();
                imports.push(request);
                json!({"items":[{"source_path":expected_source,"ok":ok,"destination_path":if ok {Some("dst/中文.bin")}else{None},"error":if ok {None}else{Some("destination exists")}}]})
            };
            let body = response.to_string();
            write!(stream,"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",body.len(),body).unwrap();
        }
        imports
    });
    let mut app = App::new().0;
    app.project_path = Some(root.clone());
    app.lsp_disabled = true;
    app.file_tree_operation_in_progress = true;
    app.pending_drop_file_paths = vec!["/stale-hover".into()];
    let mut tasks = vec![];
    for (project, directory) in [
        (root.clone(), format!("{root}/dst")),
        (root.clone(), root.clone()),
        (format!("{root}/branch"), format!("{root}/branch")),
    ] {
        app.project_path = Some(project.clone());
        tasks.push(
            super::super::file_tree::handle(
                &mut app,
                P::FileTreeExternalDropRequested {
                    project_path: project,
                    source_path: source.clone(),
                    target_directory: directory,
                },
            )
            .unwrap(),
        );
    }
    assert!(app.pending_drop_file_paths.is_empty());
    let _ = crate::app::message::view::update(&mut app, ViewMessage::HoveredFilesLeft);
    let _ = crate::app::message::view::update(&mut app, ViewMessage::GlobalMouseReleased);
    app.project_path = Some("/other-project".into());
    app.files = vec!["/other-project/keep".into()];
    for task in tasks {
        let mut replies = messages(task).await;
        assert_eq!(replies.len(), 1);
        let Message::Project(reply @ P::FileTreeImportCompleted { .. }) = replies.remove(0) else {
            panic!("import callback")
        };
        let _ = super::super::file_tree::handle(&mut app, reply);
    }
    assert!(app.active_toast.as_ref().unwrap().message.contains("destination exists"));
    assert_eq!(app.files, vec!["/other-project/keep"]);
    assert_eq!(std::fs::read(&source).unwrap(), [0, 255, 9]);
    assert_eq!(server.join().unwrap().len(), 3);
}
