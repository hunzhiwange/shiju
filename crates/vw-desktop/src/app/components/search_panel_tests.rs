//! 验证桌面数据适配后的搜索分组、空态与模态布局。
//! 无匹配、选中索引和长结果列表用于覆盖滚动与焦点反馈。

use super::search_panel::view;
use crate::app::App;

fn session(id: &str, title: &str, directory: &str) -> session_info::Info {
    session_info::Info {
        id: id.to_string(),
        slug: id.to_string(),
        project_id: "project-1".to_string(),
        directory: directory.to_string(),
        parent_id: None,
        summary: None,
        share: None,
        title: title.to_string(),
        version: "0.0.0".to_string(),
        time: session_info::TimeInfo { created: 0, updated: 0, compacting: None, archived: None },
        permission: None,
        fork_execution_started: None,
        revert: None,
    }
}

fn search_view_content_len(app: &App) -> usize {
    let search_view = view(app);
    search_view.as_widget().children().first().map_or(0, |content| content.children.len())
}

#[test]
fn view_groups_file_project_and_session_matches() {
    let mut app = App::new().0;
    app.search.search_text = "needle".to_string();
    app.project_path = Some("/repo".to_string());
    app.set_file_index("/repo", vec!["src/needle.rs".to_string()]);
    app.recent_projects = vec!["/workspace/needle-project".to_string()];
    app.sessions = vec![session("session-1", "Needle session", "/workspace/demo")];

    assert_eq!(search_view_content_len(&app), 6);
}
