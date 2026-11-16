use crate::app::state::{ManagedWorktreeGroup, ManagedWorktreesState};
use vw_api_project::ProjectDto;

fn project(id: &str) -> ProjectDto {
    serde_json::from_value(serde_json::json!({
        "id":id,"name":format!("中文项目-{id}"),"directory":format!("/项目/{id}"),
        "display_path":format!("/项目/{id}"),"status":"ready","created_at_ms":1,"updated_at_ms":2,
        "git":{"is_repo":true,"has_uncommitted_changes":false}
    }))
    .unwrap()
}
fn group(id: &str) -> ManagedWorktreeGroup {
    ManagedWorktreeGroup { project: project(id), items: vec![], collapsed: false, error: None }
}
#[test]
fn worktree_managed_ui_opens_chat_in_selected_existing_checkout() {
    let mut app = crate::app::App::new().0;
    let root = tempfile::tempdir().unwrap();
    let primary = root.path().join("primary");
    let checkout = root.path().join("原工作树");
    let repo = git2::Repository::init(&primary).unwrap();
    let tree_id = repo.index().unwrap().write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let signature = git2::Signature::now("Fixture", "fixture@example.test").unwrap();
    repo.commit(Some("HEAD"), &signature, &signature, "initial", &tree, &[]).unwrap();
    repo.worktree("existing", &checkout, None).unwrap();
    let before = repo.worktrees().unwrap().len();
    let mut project = project("same");
    project.directory = primary.to_string_lossy().into_owned();
    app.navigation_tabs.project_path = Some(project.directory.clone());
    app.project_id = Some(project.id.as_ref().to_owned());
    app.active_session_id = Some("previous".into());
    let _ = crate::app::message::project::open_existing_checkout(
        &mut app,
        project,
        checkout.to_string_lossy().into_owned(),
    );
    assert!(app.active_session_id.is_none());
    let draft = app.current_session_runtime().draft.unwrap();
    assert_eq!(draft.directory, checkout.to_string_lossy());
    assert_eq!(repo.worktrees().unwrap().len(), before);
    assert_eq!(draft.project_path, primary.to_string_lossy());
    assert!(!app.new_session_creating);
    assert!(!app.current_session_runtime().request_lifecycle.is_requesting);
    assert!(app.chat.is_empty());
}
#[test]
fn worktree_managed_ui_switches_project_before_draft() {
    let mut app = crate::app::App::new().0;
    app.navigation_tabs.project_path = Some("/旧项目".into());
    app.project_id = Some("old".into());
    let _ = crate::app::message::project::open_existing_checkout(
        &mut app,
        project("other"),
        "/工作树/跨项目".into(),
    );
    assert_eq!(app.navigation_tabs.project_path.as_deref(), Some("/项目/other"));
    assert_eq!(app.project_id.as_deref(), Some("other"));
    let draft = app.current_session_runtime().draft.unwrap();
    assert_eq!(draft.project_path, "/项目/other");
    assert_eq!(draft.directory, "/工作树/跨项目");
    assert!(!app.new_session_creating);
    assert!(!app.current_session_runtime().request_lifecycle.is_requesting);
}


fn item(id: &str) -> vw_api_worktree::ManagedWorktreeDto {
    serde_json::from_value(serde_json::json!({
        "worktree":{"id":"checkout", "project_id":id,"name":"现有工作树", "branch":"feature",
            "directory":"/工作树/很长的中文路径用于验证换行和字形整形/原目录", "status":"ready", "created_at_ms":1,"updated_at_ms":2},
        "title":"中文聊天标题，用于检查很长的动态文本是否安全渲染", "sessions":[],"protection_reasons":[], "recovered_from":null
    })).unwrap()
}
fn managed_app() -> crate::app::App {
    let mut app = crate::app::App::new().0;
    app.navigation_tabs.project_path = Some("/项目/same".into());
    app.project_id = Some("same".into());
    let mut group = group("same");
    group.items = vec![item("same")];
    app.worktree_settings.managed.groups = vec![group];
    app
}
fn update(
    app: &mut crate::app::App,
    event: crate::app::message::settings::worktrees::ManagedMessage,
) {
    let _ = crate::app::message::settings::worktrees::update(
        app,
        crate::app::message::settings::worktrees::WorktreeSettingsMessage::Managed(event),
    );
}
#[test]
fn worktree_managed_ui_deduplicates_and_preserves_session_on_failure() {
    use crate::app::message::settings::worktrees::ManagedMessage as Event;
    let mut app = managed_app();
    app.active_session_id = Some("keep".into());
    let identity = crate::app::state::worktree_identity(&app);
    let navigation = app.session_view_cache.navigation_generation;
    update(&mut app, Event::Open { project: "same".into(), worktree: "checkout".into() });
    let pending = app.worktree_settings.managed.opening.clone();
    assert!(pending.is_some());
    update(&mut app, Event::Open { project: "other".into(), worktree: "another".into() });
    assert_eq!(app.worktree_settings.managed.opening, pending);
    update(
        &mut app,
        Event::Prepared {
            identity,
            navigation,
            target: pending.unwrap(),
            result: Err("网关不可达".into()),
        },
    );
    assert_eq!(app.active_session_id.as_deref(), Some("keep"));
    assert_eq!(app.navigation_tabs.project_path.as_deref(), Some("/项目/same"));
    assert!(app.worktree_settings.managed.opening.is_none());
    assert_eq!(app.worktree_settings.managed.error.as_deref(), Some("网关不可达"));
}
#[test]
fn worktree_managed_ui_rejects_stale_refresh_and_navigation() {
    use crate::app::message::settings::worktrees::ManagedMessage as Event;
    let mut app = managed_app();
    let identity = crate::app::state::worktree_identity(&app);
    update(&mut app, Event::Refresh);
    let old = app.worktree_settings.managed.generation;
    update(&mut app, Event::Refresh);
    update(
        &mut app,
        Event::Loaded { identity: identity.clone(), generation: old, result: Ok(vec![]) },
    );
    assert_eq!(app.worktree_settings.managed.groups.len(), 1);
    assert!(app.worktree_settings.managed.loading);
    let generation = app.worktree_settings.managed.generation;
    update(
        &mut app,
        Event::Loaded {
            identity: identity.clone(), generation, result: Err("刷新失败".into())
        },
    );
    assert!(!app.worktree_settings.managed.loading);
    assert_eq!(app.worktree_settings.managed.groups.len(), 1);
    let navigation = app.session_view_cache.navigation_generation;
    update(&mut app, Event::Open { project: "same".into(), worktree: "checkout".into() });
    app.session_view_cache.navigation_generation += 1;
    app.active_session_id = Some("new-navigation".into());
    update(
        &mut app,
        Event::Prepared {
            identity,
            navigation,
            target: ("same".into(), "checkout".into()),
            result: Ok((project("same"), "/工作树/旧响应".into())),
        },
    );
    assert_eq!(app.active_session_id.as_deref(), Some("new-navigation"));
}
#[test]
fn worktree_managed_ui_preflight_is_scoped_and_never_deletes() {
    use crate::app::message::settings::worktrees::ManagedMessage as Event;
    use vw_api_worktree::{
        WorktreeDeletionPreflight, WorktreeProtectionReason,
    };
    let mut app = managed_app();
    let identity = crate::app::state::worktree_identity(&app);
    app.worktree_settings.managed.groups[0].items[0].protection_reasons =
        vec![WorktreeProtectionReason::Pinned];
    update(&mut app, Event::Preflight { project: "same".into(), worktree: "checkout".into() });
    assert!(app.worktree_settings.managed.preflight_target.is_none());
    app.worktree_settings.managed.groups[0].items[0].protection_reasons.clear();
    update(&mut app, Event::Preflight { project: "same".into(), worktree: "checkout".into() });
    let generation = app.worktree_settings.managed.generation;
    update(
        &mut app,
        Event::PreflightLoaded {
            identity,
            generation,
            target: ("same".into(), "checkout".into()),
            result: Ok(WorktreeDeletionPreflight {
                fingerprint: "only-a-preflight".into(),
                expires_at_ms: u64::MAX,
                confirmation_required: true,
                has_changes: true,
                has_ignored_files: true,
                protection_reasons: vec![],
            }),
        },
    );
    assert!(app.worktree_settings.managed.preflight.is_some());
    assert_eq!(app.worktree_settings.managed.groups[0].items.len(), 1);
    assert!(app.current_session_runtime().draft.is_none());
}





#[test]
fn worktree_managed_ui_returns_to_project_screen_before_showing_draft() {
    let mut app = managed_app();
    app.navigation_tabs.screen = crate::app::Screen::Home;
    app.navigation_panels.show_system_settings = true;
    let _ = crate::app::message::project::open_existing_checkout(
        &mut app,
        project("same"),
        "/工作树/原目录".into(),
    );
    assert!(matches!(app.navigation_tabs.screen, crate::app::Screen::Project));
    assert_eq!(app.navigation_tabs.active_tab_id.as_deref(), Some("project_/项目/same"));
    assert!(!app.navigation_panels.show_system_settings);
    assert_eq!(app.current_session_runtime().draft.unwrap().directory, "/工作树/原目录");
}
