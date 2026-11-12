//! Run the real Iced task and HTTP client in an isolated configuration process.
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

#[tokio::test]
async fn file_tree_move_drop_http_counts_requests_for_both_release_orders_and_conflict() {
    const CHILD: &str = "SHIJU_TREE_MOVE_TEST_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let home = tempfile::tempdir().unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", &format!("{}::file_tree_move_drop_http_counts_requests_for_both_release_orders_and_conflict", module_path!().split_once("::").unwrap().1), "--test-threads=1"])
            .env(CHILD, "1").env("HOME", home.path())
            .output().unwrap();
        assert!(
            output.status.success()
                && String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed"),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let root_dir = tempfile::tempdir().unwrap();
    let root = root_dir.path().to_string_lossy().into_owned();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port();
    let mut config = vw_config_types::ui::GatewayClientSystemSettingsConfig::default();
    config.set_servers(
        vec![vw_config_types::ui::GatewayClientServerConfig {
            id: "move-test".into(),
            host: "127.0.0.1".into(),
            port,
            ..Default::default()
        }],
        "move-test".into(),
    );
    let config_path = vw_config_types::paths::home_config_dir(std::env::var_os("HOME").unwrap())
        .join("gateway-client-bootstrap.json");
    std::fs::create_dir_all(config_path.parent().unwrap()).unwrap();
    std::fs::write(config_path, serde_json::to_vec(&config).unwrap()).unwrap();
    let server_root = root.clone();
    let server = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        let mut recorded = vec![];
        while recorded.len() < 12 && std::time::Instant::now() < deadline {
            let (mut stream, _) = match listener.accept() {
                Ok(connection) => connection,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                    continue;
                }
                Err(e) => panic!("{e}"),
            };
            stream.set_nonblocking(false).unwrap();
            stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
            let mut bytes = vec![];
            let mut buf = [0; 4096];
            loop {
                let count = stream.read(&mut buf).unwrap();
                assert_ne!(count, 0);
                bytes.extend_from_slice(&buf[..count]);
                if let Some(end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&bytes[..end]).to_ascii_lowercase();
                    let length: usize = headers
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length:"))
                        .unwrap_or("0")
                        .trim()
                        .parse()
                        .unwrap();
                    if bytes.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            let end = bytes.windows(4).position(|window| window == b"\r\n\r\n").unwrap();
            let header = String::from_utf8_lossy(&bytes[..end]);
            let request: serde_json::Value = serde_json::from_slice(&bytes[end + 4..]).unwrap();
            let resolve = header.starts_with("POST /v1/projects/resolve ");
            let response = if resolve {
                json!({"project": {"id":"project-test","name":"test","directory":server_root,"display_path":server_root,"status":"ready","created_at_ms":0,"updated_at_ms":0,"git":{"is_repo":false,"has_uncommitted_changes":false}}})
            } else {
                assert!(header.starts_with("POST /v1/files/move "), "{header}");
                assert_eq!(request["overwrite"], false);
                assert_eq!(
                    request["to_path"],
                    format!("dst/{}", request["from_path"].as_str().unwrap())
                );
                json!({"ok": recorded.len() != 5 && request["from_path"] != "b.txt", "message":"destination exists"})
            };
            let body = response.to_string();
            write!(stream, "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}", body.len(), body).unwrap();
            recorded.push((resolve, request));
        }
        recorded
    });
    for scenario in 0..3 {
        let mut app = App::new().0;
        app.project_path = Some(root.clone());
        app.lsp_disabled = true;
        let source = format!("{root}/a.txt");
        let mut tab = super::preview_tab(&source);
        tab.content = "unsaved".into();
        tab.editor = crate::app::components::editor::Editor::new("unsaved", "txt");
        tab.is_dirty = true;
        app.preview_tabs.push(tab);
        let _ = super::super::handle(
            &mut app,
            P::FileTreeInternalDragStart {
                project_path: root.clone(),
                source_path: source.clone(),
            },
        );
        let _ = super::super::handle(
            &mut app,
            P::FileTreeDropTargetChanged(Some(format!("{root}/dst"))),
        );
        app.input_drop_hovered = true;
        let task = if scenario == 0 {
            let task =
                crate::app::message::view::update(&mut app, ViewMessage::GlobalMouseReleased);
            let duplicate = super::super::handle(&mut app, P::FileTreeDragEnd).unwrap();
            assert!(messages(duplicate).await.is_empty());
            task
        } else if scenario == 1 {
            let task = super::super::handle(
                &mut app,
                P::FileTreeDropRequested {
                    project_path: root.clone(),
                    source_path: source.clone(),
                    target_directory: format!("{root}/dst"),
                },
            )
            .unwrap();
            let duplicate = super::super::handle(&mut app, P::FileTreeDragEnd).unwrap();
            assert!(messages(duplicate).await.is_empty());
            let duplicate =
                crate::app::message::view::update(&mut app, ViewMessage::GlobalMouseReleased);
            assert!(messages(duplicate).await.is_empty());
            task
        } else {
            let task = super::super::handle(&mut app, P::FileTreeDragEnd).unwrap();
            let duplicate =
                crate::app::message::view::update(&mut app, ViewMessage::GlobalMouseReleased);
            assert!(messages(duplicate).await.is_empty());
            task
        };
        let duplicate = super::super::handle(
            &mut app,
            P::FileTreeDropRequested {
                project_path: root.clone(),
                source_path: source.clone(),
                target_directory: format!("{root}/dst"),
            },
        )
        .unwrap();
        assert!(messages(duplicate).await.is_empty());
        let mut replies = messages(task).await;
        assert_eq!(replies.len(), 1);
        let Message::Project(reply @ P::FileTreeMoveCompleted { .. }) = replies.remove(0) else {
            panic!("move callback")
        };
        let _ = super::super::handle(&mut app, reply);
        assert_eq!(
            app.preview_tabs[0].path,
            if scenario < 2 { format!("{root}/dst/a.txt") } else { source }
        );
        assert_eq!(app.preview_tabs[0].content, "unsaved");
        assert!(app.preview_tabs[0].is_dirty);
        assert_eq!(app.preview_tabs[0].editor.content(), "unsaved");
        if scenario == 2 {
            assert!(app.active_toast.as_ref().unwrap().message.contains("destination exists"));
        }
        assert!(!app.file_tree_operation_in_progress);
        assert!(app.pending_drop_file_paths.is_empty());
    }
    let mut app = App::new().0;
    app.project_path = Some(root.clone());
    app.lsp_disabled = true;
    let sources = ["a.txt", "b.txt", "c.txt"].map(|path| format!("{root}/{path}"));
    app.set_file_index(&root, sources.to_vec());
    for path in &sources {
        let _ = super::super::handle(
            &mut app,
            P::FileTreeSelected {
                project_path: root.clone(),
                path: path.clone(),
                directory: false,
                modifiers: iced::keyboard::Modifiers::COMMAND,
            },
        );
    }
    let _ = super::super::handle(
        &mut app,
        P::FileTreeInternalDragStart {
            project_path: root.clone(),
            source_path: sources[1].clone(),
        },
    );
    assert_eq!(app.dragging_file_paths, sources);
    let _ =
        super::super::handle(&mut app, P::FileTreeDropTargetChanged(Some(format!("{root}/dst"))));
    let task = crate::app::message::view::update(&mut app, ViewMessage::GlobalMouseReleased);
    assert!(messages(super::super::handle(&mut app, P::FileTreeDragEnd).unwrap()).await.is_empty());
    let mut replies = messages(task).await;
    assert_eq!(replies.len(), 1);
    let Message::Project(reply @ P::FileTreeBatchMoveCompleted { .. }) = replies.remove(0) else {
        panic!("batch move callback")
    };
    let P::FileTreeBatchMoveCompleted { moved_paths, errors, .. } = &reply else { unreachable!() };
    assert_eq!(moved_paths.len(), 2);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("b.txt"));
    let _ = super::super::handle(&mut app, reply);
    assert!(app.file_tree_selection.contains(&root, &format!("{root}/dst/a.txt")));
    assert!(app.file_tree_selection.contains(&root, &sources[1]));
    assert!(app.file_tree_selection.contains(&root, &format!("{root}/dst/c.txt")));
    assert!(app.active_toast.as_ref().unwrap().message.contains("destination exists"));
    assert!(!app.file_tree_operation_in_progress);
    assert!(app.pending_drop_file_paths.is_empty());
    let requests = server.join().unwrap();
    assert_eq!(requests.len(), 12);
    assert_eq!(requests.iter().filter(|(resolve, _)| !resolve).count(), 6);
}
