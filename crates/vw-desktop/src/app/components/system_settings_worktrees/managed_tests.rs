use crate::app::state::{ManagedWorktreeGroup, ManagedWorktreesState};
use vw_gateway_client::vw_api_types::project::ProjectDto;

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
    app.project_path = Some(project.directory.clone());
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
    assert!(!app.current_session_runtime().is_requesting);
    assert!(app.chat.is_empty());
}
#[test]
fn worktree_managed_ui_switches_project_before_draft() {
    let mut app = crate::app::App::new().0;
    app.project_path = Some("/旧项目".into());
    app.project_id = Some("old".into());
    let _ = crate::app::message::project::open_existing_checkout(
        &mut app,
        project("other"),
        "/工作树/跨项目".into(),
    );
    assert_eq!(app.project_path.as_deref(), Some("/项目/other"));
    assert_eq!(app.project_id.as_deref(), Some("other"));
    let draft = app.current_session_runtime().draft.unwrap();
    assert_eq!(draft.project_path, "/项目/other");
    assert_eq!(draft.directory, "/工作树/跨项目");
    assert!(!app.new_session_creating);
    assert!(!app.current_session_runtime().is_requesting);
}
#[test]
fn worktree_managed_ui_refresh_keeps_groups_stable() {
    let mut state = ManagedWorktreesState::default();
    let mut first = group("b");
    first.collapsed = true;
    state.groups = vec![first, group("a")];
    state.replace_groups(vec![group("a"), group("b"), group("c")]);
    assert_eq!(
        state.groups.iter().map(|g| g.project.id.as_ref()).collect::<Vec<_>>(),
        ["b", "a", "c"]
    );
    assert!(state.groups[0].collapsed);
}

fn item(id: &str) -> vw_gateway_client::vw_api_types::worktree::ManagedWorktreeDto {
    serde_json::from_value(serde_json::json!({
        "worktree":{"id":"checkout", "project_id":id,"name":"现有工作树", "branch":"feature",
            "directory":"/工作树/很长的中文路径用于验证换行和字形整形/原目录", "status":"ready", "created_at_ms":1,"updated_at_ms":2},
        "title":"中文聊天标题，用于检查很长的动态文本是否安全渲染", "sessions":[],"protection_reasons":[], "recovered_from":null
    })).unwrap()
}
fn managed_app() -> crate::app::App {
    let mut app = crate::app::App::new().0;
    app.project_path = Some("/项目/same".into());
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
    let identity = crate::app::state::WorktreeSettingsIdentity::current(&app);
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
    assert_eq!(app.project_path.as_deref(), Some("/项目/same"));
    assert!(app.worktree_settings.managed.opening.is_none());
    assert_eq!(app.worktree_settings.managed.error.as_deref(), Some("网关不可达"));
}
#[test]
fn worktree_managed_ui_rejects_stale_refresh_and_navigation() {
    use crate::app::message::settings::worktrees::ManagedMessage as Event;
    let mut app = managed_app();
    let identity = crate::app::state::WorktreeSettingsIdentity::current(&app);
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
    use vw_gateway_client::vw_api_types::worktree::{
        WorktreeDeletionPreflight, WorktreeProtectionReason,
    };
    let mut app = managed_app();
    let identity = crate::app::state::WorktreeSettingsIdentity::current(&app);
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
fn worktree_managed_ui_partial_error_retains_rows_and_collapsed_groups() {
    let mut state = ManagedWorktreesState::default();
    let mut old = group("same");
    old.items = vec![item("same")];
    old.collapsed = true;
    state.groups.push(old);
    let mut failed = group("same");
    failed.error = Some("部分项目加载失败".into());
    state.replace_groups(vec![failed]);
    assert!(state.groups[0].collapsed);
    assert_eq!(state.groups[0].items.len(), 1);
    assert_eq!(state.groups[0].error.as_deref(), Some("部分项目加载失败"));
}
#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn worktree_managed_ui_renders_long_chinese_and_errors_in_both_themes() {
    use crate::app::i18n::I18n;
    use iced::advanced::{
        clipboard,
        renderer::{Headless, Style},
    };
    use iced::{Font, Renderer, Size, Theme, mouse};
    use iced_runtime::user_interface::{Cache, UserInterface};
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let mut renderer =
        Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
            .await
            .unwrap();
    let mut app = managed_app();
    let mut failed = group("错误项目");
    failed.error = Some("网关返回错误，原有数据保留，请刷新重试。".into());
    app.worktree_settings.managed.groups.push(failed);
    for theme in [Theme::Light, Theme::Dark] {
        let size = Size::new(640.0, 700.0);
        let mut ui = UserInterface::build(
            super::managed::view(&app.worktree_settings.managed, I18n::new(app.desktop_locale)),
            size,
            Cache::new(),
            &mut renderer,
        );
        ui.update(
            &[],
            mouse::Cursor::Unavailable,
            &mut renderer,
            &mut clipboard::Null,
            &mut Vec::new(),
        );
        ui.draw(
            &mut renderer,
            &theme,
            &Style { text_color: theme.palette().text },
            mouse::Cursor::Unavailable,
        );
        let pixels = renderer.screenshot(Size::new(640, 700), 1.0, theme.palette().background);
        assert_eq!(pixels.len(), 640 * 700 * 4);
        if let Some(root) = std::env::var_os("SHIJU_MANAGED_WORKTREE_SCREENSHOTS") {
            std::fs::create_dir_all(&root).unwrap();
            image::save_buffer(
                std::path::PathBuf::from(root).join(format!("managed-{theme:?}.png")),
                &pixels,
                640,
                700,
                image::ColorType::Rgba8,
            )
            .unwrap();
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn worktree_managed_ui_paginates_all_projects_and_keeps_partial_failure() {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint =
        vw_gateway_client::GatewayEndpoint::new("127.0.0.1", listener.local_addr().unwrap().port());
    let client = vw_gateway_client::GatewayClient::new(endpoint).unwrap();
    let server = std::thread::spawn(move || {
        listener.set_nonblocking(true).unwrap();
        let mut requests = Vec::new();
        let mut exchanges = vec![(
            "GET /v1/projects?limit=100 ".to_string(),
            200,
            serde_json::json!({"items":(0..50).map(|i|project(&format!("p{i}"))).collect::<Vec<_>>(),"next_cursor":"page2"}),
        )];
        for i in 0..50 {
            exchanges.push((
                format!("GET /v1/projects/p{i}/managed-worktrees "),
                200,
                serde_json::json!({"items":[]}),
            ));
        }
        exchanges.push((
            "GET /v1/projects?cursor=page2&limit=100 ".into(),
            200,
            serde_json::json!({"items":[project("p50"),project("p51")],"next_cursor":null}),
        ));
        exchanges.push((
            "GET /v1/projects/p50/managed-worktrees ".into(),
            500,
            serde_json::json!({"error":"fixture failure"}),
        ));
        exchanges.push((
            "GET /v1/projects/p51/managed-worktrees ".into(),
            200,
            serde_json::json!({"items":[]}),
        ));
        for (expected, status, body) in exchanges {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(e)
                        if e.kind() == std::io::ErrorKind::WouldBlock
                            && std::time::Instant::now() < deadline =>
                    {
                        std::thread::sleep(std::time::Duration::from_millis(10))
                    }
                    Err(e) => panic!("missing {expected}: {e}"),
                }
            };
            // Accepted sockets can inherit the listener's nonblocking mode on macOS.
            stream.set_nonblocking(false).unwrap();
            stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
            let mut header = Vec::new();
            while !header.ends_with(b"\r\n\r\n") {
                let mut b = [0];
                stream.read_exact(&mut b).unwrap();
                header.push(b[0]);
            }
            let header = String::from_utf8(header).unwrap();
            assert!(header.starts_with(&expected), "{header}");
            requests.push(header);
            let body = body.to_string();
            write!(stream,"HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
        }
        requests
    });
    let groups = crate::app::message::settings::worktrees::load_groups(&client).await.unwrap();
    assert_eq!(groups.len(), 52);
    assert!(groups[50].error.is_some());
    assert!(groups[51].error.is_none());
    assert_eq!(server.join().unwrap().len(), 54);
}

#[test]
fn worktree_managed_ui_returns_to_project_screen_before_showing_draft() {
    let mut app = managed_app();
    app.screen = crate::app::Screen::Home;
    app.show_system_settings = true;
    let _ = crate::app::message::project::open_existing_checkout(
        &mut app,
        project("same"),
        "/工作树/原目录".into(),
    );
    assert!(matches!(app.screen, crate::app::Screen::Project));
    assert_eq!(app.active_tab_id.as_deref(), Some("project_/项目/same"));
    assert!(!app.show_system_settings);
    assert_eq!(app.current_session_runtime().draft.unwrap().directory, "/工作树/原目录");
}
