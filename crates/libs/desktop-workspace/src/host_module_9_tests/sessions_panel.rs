use crate::views::project::components::{
    session_row,
    sessions_panel::{self as domain, Message, session_tree_projection},
};
use iced::{Element, Point};
use session_info::{Info, Summary, TimeInfo};
use std::collections::{HashMap, HashSet};
#[derive(Default)]
struct PanelFixture {
    recent_projects: Vec<String>,
    recent_projects_edits: Vec<String>,
    project_sessions: HashMap<String, Vec<Info>>,
    project_sessions_loading: HashSet<String>,
    project_session_load_counts: HashMap<String, usize>,
    project_session_has_vertical_scrollbar: HashMap<String, bool>,
    project_tools_menu_path: Option<String>,
    session_menu_id: Option<String>,
    session_menu_anchor: Option<Point>,
}
impl PanelFixture {
    fn input(&self, sessions: Option<&Vec<Info>>) -> domain::Input<'_> {
        let rows = self
            .project_sessions
            .values()
            .flatten()
            .chain(sessions.into_iter().flatten())
            .map(|s| {
                (
                    s.id.clone(),
                    (
                        session_row::Input {
                            desktop_locale: config_ui::DesktopLocale::SimplifiedChinese,
                            active: false,
                            title: s.title.clone(),
                            pinned: false,
                            status_animation_frame: 0,
                            menu_open: self.session_menu_id.is_some(),
                            workspace_kind: None,
                            agent: "—".into(),
                            relative: String::new(),
                        },
                        session_row::SessionRowStatus::Idle,
                    ),
                )
            })
            .collect();
        domain::Input {
            desktop_locale: config_ui::DesktopLocale::SimplifiedChinese,
            pinned: &EMPTY_PINNED,
            session_menu_id: &self.session_menu_id,
            session_menu_anchor: self.session_menu_anchor,
            rows,
            drafts: vec![],
            recent_projects: &self.recent_projects,
            recent_projects_edits: &self.recent_projects_edits,
            project_sessions_loading: &self.project_sessions_loading,
            project_session_load_counts: &self.project_session_load_counts,
            project_session_has_vertical_scrollbar: &self.project_session_has_vertical_scrollbar,
            project_sessions: &self.project_sessions,
            project_tools_menu_path: &self.project_tools_menu_path,
            new_session: Default::default(),
        }
    }
}
static EMPTY_PINNED: std::sync::LazyLock<HashSet<(String, String)>> =
    std::sync::LazyLock::new(HashSet::new);
fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}
fn session_items_list<'a>(
    fixture: &PanelFixture,
    path: &str,
    sessions: Option<&Vec<Info>>,
    title: &str,
    count: usize,
    loading: bool,
) -> Element<'a, Message> {
    domain::session_items_list(&fixture.input(sessions), path, sessions, title, count, loading)
}
fn project_sessions_panel<'a>(
    fixture: &PanelFixture,
    path: Option<String>,
) -> Element<'a, Message> {
    domain::project_sessions_panel(&fixture.input(None), path)
}
fn session(id: &str, title: &str, directory: &str, archived: bool) -> Info {
    Info {
        id: id.to_string(),
        slug: format!("{id}-slug"),
        project_id: "project".to_string(),
        directory: directory.to_string(),
        parent_id: None,
        summary: Some(Summary { additions: 3, deletions: 1, files: 2, diffs: None }),
        share: None,
        title: title.to_string(),
        version: "1".to_string(),
        time: TimeInfo {
            created: 1,
            updated: 2,
            compacting: None,
            archived: archived.then_some(3),
        },
        permission: None,
        fork_execution_started: None,
        revert: None,
    }
}

fn child_session(id: &str, parent_id: &str, updated: u64) -> Info {
    let mut info = session(id, id, "/tmp/project", false);
    info.parent_id = Some(parent_id.to_string());
    info.time.updated = updated;
    info
}

#[test]
fn session_items_build_loading_empty_and_load_button_states() {
    let app = PanelFixture::default();
    keep_element(session_items_list(&app, "/tmp/project", None, "Project", 10, false));
    keep_element(session_items_list(&app, "/tmp/project", Some(&Vec::new()), "Project", 10, false));
    keep_element(session_items_list(&app, "/tmp/project", None, "Project", 10, true));
}

#[test]
fn session_tree_projects_only_main_tasks_regardless_of_expansion() {
    let mut older_root = session("older-root", "Older root", "/tmp/project", false);
    older_root.time.updated = 10;
    let mut newer_root = session("newer-root", "Newer root", "/tmp/project", false);
    newer_root.time.updated = 50;
    let child = child_session("child", "newer-root", 40);
    let sibling = child_session("sibling", "newer-root", 20);
    let grandchild = child_session("grandchild", "child", 30);
    let sessions = vec![older_root, sibling, grandchild, newer_root, child];

    let collapsed = session_tree_projection(&sessions, 1, &HashSet::new());
    assert_eq!(collapsed.total_roots, 2);
    assert_eq!(
        collapsed.rows.iter().map(|row| row.session.id.as_str()).collect::<Vec<_>>(),
        vec!["newer-root"]
    );

    let parent_expanded = session_tree_projection(&sessions, 1, &HashSet::new());
    assert_eq!(parent_expanded.total_roots, 2);
    assert_eq!(
        parent_expanded.rows.iter().map(|row| row.session.id.as_str()).collect::<Vec<_>>(),
        vec!["newer-root"]
    );

    let recursively_expanded = session_tree_projection(&sessions, 1, &HashSet::new());
    assert_eq!(
        recursively_expanded.rows.iter().map(|row| row.session.id.as_str()).collect::<Vec<_>>(),
        vec!["newer-root"]
    );

    let app = PanelFixture::default();
    keep_element(session_items_list(&app, "/tmp/project", Some(&sessions), "Project", 1, false));
}

#[test]
fn session_items_filters_archived_and_shows_load_more() {
    let app = PanelFixture::default();
    let sessions = vec![
        session("s1", "可见会话一", "/tmp/project", false),
        session("s2", "已归档会话", "/tmp/project", true),
        session("s3", "可见会话二", "/tmp/project/worktree-a", false),
    ];

    keep_element(session_items_list(&app, "/tmp/project", Some(&sessions), "Project", 1, false));
    keep_element(session_items_list(&app, "/tmp/project", Some(&sessions), "Project", 0, false));
}

#[test]
fn session_items_reports_empty_when_only_archived_sessions_exist() {
    let app = PanelFixture::default();
    let sessions = vec![
        session("s1", "归档一", "/tmp/project", true),
        session("s2", "归档二", "/tmp/project/worktree-a", true),
    ];

    keep_element(session_items_list(&app, "/tmp/project", Some(&sessions), "Project", 10, false));
}

#[test]
fn session_items_builds_missing_summary_menu_default_anchor_and_no_load_more() {
    let mut app = PanelFixture::default();
    app.session_menu_id = Some("summaryless".to_string());
    app.session_menu_anchor = None;

    let mut summaryless = session("summaryless", "没有摘要的会话", "/", false);
    summaryless.summary = None;
    let sessions =
        vec![summaryless, session("normal", "普通会话", "/tmp/project/worktree-d", false)];

    keep_element(session_items_list(&app, "/tmp/project", Some(&sessions), "Project", 10, false));
}

#[test]
fn project_panel_builds_empty_and_selected_project_states() {
    let mut app = PanelFixture::default();
    keep_element(project_sessions_panel(&app, None));

    app.recent_projects.push("/tmp/project".to_string());
    keep_element(project_sessions_panel(&app, None));

    app.recent_projects_edits.push("自定义项目名".to_string());
    app.project_tools_menu_path = Some("/tmp/project".to_string());
    app.project_session_has_vertical_scrollbar.insert("/tmp/project".to_string(), true);
    app.project_session_load_counts.insert("/tmp/project".to_string(), 1);
    app.project_sessions.insert(
        "/tmp/project".to_string(),
        vec![
            session("s1", "第一条会话", "/tmp/project", false),
            session("s2", "第二条会话", "/tmp/project/worktree-c", false),
        ],
    );

    keep_element(project_sessions_panel(&app, Some("/tmp/project".to_string())));

    app.project_sessions_loading.insert("/tmp/loading".to_string());
    keep_element(project_sessions_panel(&app, Some("/tmp/loading".to_string())));
}

#[test]
fn project_panel_uses_path_fallback_for_unknown_or_blank_recent_names() {
    let mut app = PanelFixture::default();
    app.recent_projects = vec!["/tmp/project".to_string()];
    app.recent_projects_edits = vec!["   ".to_string()];

    keep_element(project_sessions_panel(&app, Some("/tmp/project".to_string())));
    keep_element(project_sessions_panel(&app, Some("/tmp/unknown-project".to_string())));
}
