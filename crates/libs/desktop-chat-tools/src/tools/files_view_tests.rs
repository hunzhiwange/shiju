//! 验证文件工具总路由对读取、搜索、编辑和写入状态的视图分派。
//! 非文件工具和无路径普通调用应返回无视图，错误与运行态编辑仍需保留卡片。

use super::files_view::tool_files_view;
use super::view_fixture::TestApp as App;
type Message = crate::card_input::Message<()>;

fn test_app() -> App {
    let mut app = App::new().0;
    app.project_path = Some("/tmp/vibe-window".to_string());
    app
}

fn keep_element(element: iced::Element<'_, Message>) {
    std::hint::black_box(element);
}

#[test]
fn files_view_context_names_are_stable() {
    assert_eq!("FilesViewContext", "FilesViewContext");
}

#[test]
fn tool_files_view_rejects_invalid_or_skipped_tools() {
    let app = test_app();

    assert!(tool_files_view(&app.files(), 0, 0, "not a tool", &super::view_fixture::diff).is_none());
    assert!(tool_files_view(&app.files(), 0, 0, "tool \n{}", &super::view_fixture::diff).is_none());
    assert!(tool_files_view(&app.files(), 0, 0, "tool read\nnot json", &super::view_fixture::diff).is_none());
    assert!(
        tool_files_view(
            &app.files(),
            0,
            0,
            r#"tool apply_patch
{"input":"{}","status":"completed"}"#, &super::view_fixture::diff)
        .is_none()
    );
    assert!(
        tool_files_view(
            &app.files(),
            0,
            0,
            r#"tool git_operations
{"input":"{\"operation\":\"diff\"}","status":"completed"}"#, &super::view_fixture::diff)
        .is_none()
    );
}

#[test]
fn tool_files_view_rejects_plain_tool_without_files() {
    let app = test_app();

    assert!(tool_files_view(&app.files(), 0, 0, "tool bash\n{\"status\":\"completed\"}", &super::view_fixture::diff).is_none());
}

#[test]
fn tool_files_view_builds_read_search_and_write_views() {
    let mut app = test_app();
    app.tool_files_filter = "main".to_string();

    let read = tool_files_view(
        &app.files(),
        1,
        2,
        r#"tool read
{"input":"{\"path\":\"src/main.rs\",\"offset\":2,\"limit\":5}","status":"completed"}"#, &super::view_fixture::diff)
    .expect("read view");
    keep_element(read);

    let search = tool_files_view(
        &app.files(),
        1,
        3,
        r#"tool grep
{"output":"- src/main.rs\n- src/lib.rs","status":"completed"}"#, &super::view_fixture::diff)
    .expect("search view");
    keep_element(search);

    let write = tool_files_view(
        &app.files(),
        1,
        4,
        r#"tool write
{"input":"{\"path\":\"src/main.rs\",\"content\":\"fn main() {}\"}","status":"completed"}"#, &super::view_fixture::diff)
    .expect("write view");
    keep_element(write);
}

#[test]
fn tool_files_view_keeps_error_and_running_edit_cards() {
    let app = test_app();

    let running = tool_files_view(
        &app.files(),
        2,
        1,
        r#"tool write
{"input":"{\"path\":\"src/main.rs\"}","status":"running"}"#, &super::view_fixture::diff)
    .expect("running write view");
    keep_element(running);

    let failed = tool_files_view(
        &app.files(),
        2,
        2,
        r#"tool read
{"input":"{\"path\":\"src/main.rs\"}","status":"error","error":"permission denied"}"#, &super::view_fixture::diff)
    .expect("error read view");
    keep_element(failed);
}

#[test]
fn tool_files_view_keeps_empty_search_and_edit_like_views() {
    let app = test_app();

    let search = tool_files_view(
        &app.files(),
        3,
        1,
        r#"tool grep
{"output":"","status":"completed"}"#, &super::view_fixture::diff)
    .expect("empty search view");
    keep_element(search);

    let write = tool_files_view(
        &app.files(),
        3,
        2,
        r#"tool write
{"input":"{}","status":"completed"}"#, &super::view_fixture::diff)
    .expect("empty write view");
    keep_element(write);
}

#[test]
fn tool_files_view_handles_denied_and_write_alias_views() {
    let app = test_app();

    let denied = tool_files_view(
        &app.files(),
        4,
        1,
        r#"tool read
{"input":"{\"path\":\"src/main.rs\"}","status":"denied","error":"permission denied"}"#, &super::view_fixture::diff)
    .expect("denied read view");
    keep_element(denied);

    let file_write = tool_files_view(
        &app.files(),
        4,
        2,
        r#"tool file_write
{"input":"{\"path\":\"src/write.rs\",\"content\":\"hello\"}","status":"completed"}"#, &super::view_fixture::diff)
    .expect("file_write view");
    keep_element(file_write);

    let file_edit = tool_files_view(
        &app.files(),
        4,
        3,
        r#"tool file_edit
{"input":"{\"path\":\"src/edit.rs\"}","status":"completed"}"#, &super::view_fixture::diff)
    .expect("file_edit view");
    keep_element(file_edit);

    let notebook_edit = tool_files_view(
        &app.files(),
        4,
        4,
        r#"tool notebook_edit
{"input":"{\"path\":\"notebooks/demo.ipynb\"}","status":"completed"}"#, &super::view_fixture::diff)
    .expect("notebook_edit view");
    keep_element(notebook_edit);
}
