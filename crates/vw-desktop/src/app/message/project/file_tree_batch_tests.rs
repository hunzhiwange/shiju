use super::*;
use crate::app::message::project::FileTreeAction;
use iced::keyboard::Modifiers;

fn selected_app() -> App {
    let mut app = App::new().0;
    app.project_path = Some("/project".into());
    app.set_file_index("/project", vec!["/project/a.txt".into(), "/project/z.txt".into()]);
    for path in ["/project/z.txt", "/project/a.txt"] {
        let _ = super::super::handle(
            &mut app,
            ProjectMessage::FileTreeSelected {
                project_path: "/project".into(),
                path: path.into(),
                directory: false,
                modifiers: Modifiers::COMMAND,
            },
        );
    }
    right_click(&mut app, "/project/z.txt", "tree-file");
    app
}

fn right_click(app: &mut App, path: &str, source: &str) {
    let _ = super::super::handle(
        app,
        ProjectMessage::FileTreeRightClicked(path.into(), source.into(), 0.0, 0.0),
    );
}

fn action(app: &mut App, action: FileTreeAction) {
    let _ = super::super::handle(app, ProjectMessage::FileTreeAction(action));
}

#[test]
fn selected_context_preserves_multi_selection_and_single_actions_use_first_visible_item() {
    let mut app = selected_app();
    assert_eq!(menu_paths(&app), ["/project/a.txt", "/project/z.txt"]);
    action(&mut app, FileTreeAction::Rename);
    assert_eq!(app.file_tree_rename_path.as_deref(), Some("/project/a.txt"));
    assert_eq!(app.file_tree_selection.paths.len(), 2);
    right_click(&mut app, "/project/z.txt", "changes-file");
    assert_eq!(menu_paths(&app), ["/project/z.txt"]);
}

#[test]
fn right_clicking_unselected_item_replaces_selection() {
    let mut app = selected_app();
    app.set_file_index(
        "/project",
        vec!["/project/a.txt".into(), "/project/z.txt".into(), "/project/b.txt".into()],
    );
    right_click(&mut app, "/project/b.txt", "tree-file");
    assert_eq!(menu_paths(&app), ["/project/b.txt"]);
    assert_eq!(app.file_tree_selection.paths.len(), 1);
}

#[test]
fn batch_delete_confirmation_captures_all_paths_and_cancel_preserves_files() {
    let mut app = selected_app();
    let files = app.files.clone();
    action(&mut app, FileTreeAction::Delete);
    let (root, paths) = app.file_tree_pending_delete.as_ref().unwrap();
    assert_eq!(root, "/project");
    assert_eq!(paths, &["/project/a.txt", "/project/z.txt"]);
    assert_eq!(app.files, files);
    let _ = super::super::handle(&mut app, ProjectMessage::FileTreeDeleteCanceled);
    assert!(app.file_tree_pending_delete.is_none());
    assert_eq!(app.files, files);
}

#[test]
fn copy_cut_and_chat_actions_include_all_selected_items() {
    let mut app = selected_app();
    for operation in [FileTreeAction::Copy, FileTreeAction::Cut] {
        right_click(&mut app, "/project/z.txt", "tree-file");
        action(&mut app, operation);
        assert_eq!(
            app.file_tree_clipboard.as_ref().unwrap().src_paths,
            ["/project/a.txt", "/project/z.txt"]
        );
    }
    right_click(&mut app, "/project/z.txt", "tree-file");
    action(&mut app, FileTreeAction::AddToChat);
    let input = app.current_session_runtime().input_editor.text();
    assert!(input.contains("@a.txt "));
    assert!(input.contains("@z.txt "));
}

#[test]
fn recursive_batches_remove_descendants_without_matching_sibling_prefixes() {
    let paths = ["/project/src/a.txt", "/project/src-old/b.txt", "/project/src", "/project/src"]
        .into_iter()
        .map(str::to_string)
        .collect();
    assert_eq!(top_level_paths(paths), ["/project/src-old/b.txt", "/project/src"]);
}

#[test]
fn paste_reserves_distinct_targets_for_same_named_sources() {
    let mut app = selected_app();
    app.files.push("/project/dst/a.txt".into());
    let targets = paste_targets(
        &app,
        "/project/dst",
        &["/project/one/a.txt".into(), "/project/two/a.txt".into()],
    );
    assert_eq!(targets[0].1, "/project/dst/a copy.txt");
    assert_eq!(targets[1].1, "/project/dst/a copy 2.txt");
}

#[test]
fn partial_cut_completion_keeps_failed_sources_and_does_not_clear_new_clipboard() {
    let mut app = selected_app();
    action(&mut app, FileTreeAction::Cut);
    let clipboard = app.file_tree_clipboard.clone().unwrap();
    let complete = ProjectMessage::FileTreePasteCompleted {
        project_path: "/project".into(),
        clipboard: clipboard.clone(),
        completed_sources: vec!["/project/a.txt".into()],
        result: Err("z failed".into()),
    };
    let _ = super::super::handle(&mut app, complete.clone());
    assert_eq!(app.file_tree_clipboard.as_ref().unwrap().src_paths, ["/project/z.txt"]);
    let replacement = FileTreeClipboard {
        mode: FileTreeClipboardMode::Copy,
        src_paths: vec!["/project/new.txt".into()],
    };
    app.file_tree_clipboard = Some(replacement.clone());
    let _ = super::super::handle(&mut app, complete);
    assert_eq!(app.file_tree_clipboard, Some(replacement));
}

#[test]
fn partial_delete_completion_only_clears_successful_selection_in_original_project() {
    let mut app = selected_app();
    let complete = ProjectMessage::FileTreeDeleteCompleted {
        project_path: "/project".into(),
        deleted_paths: vec!["/project/a.txt".into()],
        result: Err("z failed".into()),
    };
    let _ = super::super::handle(&mut app, complete.clone());
    assert_eq!(
        app.file_tree_selection.paths.iter().cloned().collect::<Vec<_>>(),
        ["/project/z.txt"]
    );
    app.project_path = Some("/other".into());
    let selection = app.file_tree_selection.paths.clone();
    let _ = super::super::handle(&mut app, complete);
    assert_eq!(app.file_tree_selection.paths, selection);
}

#[tokio::test]
async fn batch_delete_http_attempts_every_root_once_and_reports_partial_failure() {
    use iced::futures::StreamExt;
    use serde_json::json;
    use std::io::{Read, Write};
    const CHILD: &str = "SHIJU_BATCH_DELETE_TEST_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let home = tempfile::tempdir().unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                &format!(
                    "{}::batch_delete_http_attempts_every_root_once_and_reports_partial_failure",
                    module_path!().split_once("::").unwrap().1
                ),
                "--test-threads=1",
            ])
            .env(CHILD, "1")
            .env("HOME", home.path())
            .output()
            .unwrap();
        assert!(
            output.status.success()
                && String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed"),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let project = tempfile::tempdir().unwrap();
    let root = project.path().to_string_lossy().into_owned();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let mut config = vw_config_types::ui::GatewayClientSystemSettingsConfig::default();
    config.set_servers(
        vec![vw_config_types::ui::GatewayClientServerConfig {
            id: "batch-delete-test".into(),
            host: "127.0.0.1".into(),
            port: listener.local_addr().unwrap().port(),
            ..Default::default()
        }],
        "batch-delete-test".into(),
    );
    let config_path = vw_config_types::paths::home_config_dir(std::env::var_os("HOME").unwrap())
        .join("gateway-client-bootstrap.json");
    std::fs::create_dir_all(config_path.parent().unwrap()).unwrap();
    std::fs::write(config_path, serde_json::to_vec(&config).unwrap()).unwrap();
    let server_root = root.clone();
    let server = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        let mut deletes = Vec::new();
        while deletes.len() < 3 && std::time::Instant::now() < deadline {
            let (mut stream, _) = match listener.accept() {
                Ok(connection) => connection,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                    continue;
                }
                Err(error) => panic!("{error}"),
            };
            stream.set_nonblocking(false).unwrap();
            stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
            let mut bytes = Vec::new();
            let mut buffer = [0; 4096];
            loop {
                let count = stream.read(&mut buffer).unwrap();
                assert_ne!(count, 0);
                bytes.extend_from_slice(&buffer[..count]);
                if let Some(end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&bytes[..end]).to_ascii_lowercase();
                    let length = headers
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length:"))
                        .unwrap_or("0")
                        .trim()
                        .parse::<usize>()
                        .unwrap();
                    if bytes.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            let end = bytes.windows(4).position(|part| part == b"\r\n\r\n").unwrap();
            let headers = String::from_utf8_lossy(&bytes[..end]);
            let request: serde_json::Value = serde_json::from_slice(&bytes[end + 4..]).unwrap();
            let response = if headers.starts_with("POST /v1/projects/resolve ") {
                json!({"project":{"id":"batch-project","name":"batch","directory":server_root,"display_path":server_root,"status":"ready","created_at_ms":0,"updated_at_ms":0,"git":{"is_repo":false,"has_uncommitted_changes":false}}})
            } else {
                assert!(headers.starts_with("POST /v1/files/delete "), "{headers}");
                let success = request["path"] != "b.txt";
                deletes.push(request);
                json!({"ok":success,"message":"denied"})
            };
            let body = response.to_string();
            write!(stream, "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}", body.len(), body).unwrap();
        }
        deletes
    });
    let mut app = App::new().0;
    app.project_path = Some(root.clone());
    let paths = ["src", "src/a.txt", "b.txt", "c.txt"].map(|path| format!("{root}/{path}"));
    app.files = paths[1..].to_vec();
    app.file_tree_pending_delete = Some((root.clone(), paths.to_vec()));
    let task = super::super::handle(&mut app, ProjectMessage::FileTreeDeleteConfirmed).unwrap();
    let mut outputs = Vec::new();
    if let Some(mut stream) = iced_runtime::task::into_stream(task) {
        while let Some(iced_runtime::Action::Output(message)) = stream.next().await {
            outputs.push(message);
        }
    }
    assert!(
        matches!(outputs.as_slice(), [Message::Project(ProjectMessage::FileTreeDeleteCompleted {
        project_path, deleted_paths, result: Err(error),
    })] if project_path == &root && deleted_paths == &vec![paths[0].clone(), paths[3].clone()]
        && error.contains("b.txt") && error.contains("denied"))
    );
    let deletes = server.join().unwrap();
    assert_eq!(deletes.len(), 3);
    assert_eq!(
        deletes.iter().map(|value| value["path"].as_str().unwrap()).collect::<Vec<_>>(),
        ["src", "b.txt", "c.txt"]
    );
    assert_eq!(deletes[0]["recursive"], true);
    assert_eq!(deletes[1]["recursive"], false);
    assert_eq!(deletes[2]["recursive"], false);
}
