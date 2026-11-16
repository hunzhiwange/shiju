//! 宿主保留：最近项目元数据、运行/排队状态转为 ProjectsInput，且点击事件映射为宿主 ProjectMessage；中文字体资源从宿主 fonts 加载。
//! 项目列表组件的测试，覆盖徽标、图标路径与最近项目元数据的展示分支。
//! fixture 同时验证主题颜色、选中态和项目入口动作能够稳定构建。


use iced::Element;

use session_info::{Info, TimeInfo};

use super::*;
use crate::app::state::SessionRuntimeState;
use crate::app::{App, Message, RecentProjectMeta};

fn session_info(id: &str, directory: &str) -> Info {
    Info {
        id: id.to_string(),
        slug: id.to_string(),
        project_id: "project".to_string(),
        directory: directory.to_string(),
        parent_id: None,
        summary: None,
        share: None,
        title: "Session".to_string(),
        version: "1".to_string(),
        time: TimeInfo { created: 1, updated: 2, compacting: None, archived: None },
        permission: None,
        fork_execution_started: None,
        revert: None,
    }
}

fn recent_meta(path: &str, icon: Option<&str>, icon_color: Option<&str>) -> RecentProjectMeta {
    RecentProjectMeta {
        path: path.to_string(),
        name: "Project".to_string(),
        task_board_settings: None,
        session_auto_refresh: true,
        session_refresh_interval_seconds: 60,
        icon: icon.map(str::to_string),
        icon_color: icon_color.map(str::to_string),
        worktree_start_command: None,
    }
}

#[test]
fn projects_list_builds_empty_recent_projects() {
    let (mut app, _task) = App::new();
    app.recent_projects.clear();
    app.recent_projects_edits.clear();
    app.recent_projects_meta.clear();

    let element: Element<'_, Message> = projects_list(&app, false);

    std::hint::black_box(element);
}


#[test]
fn projects_list_builds_with_hover_meta_and_project_attention() {
    let (mut app, _task) = App::new();
    let path = "/tmp/vibe-window-alpha".to_string();
    let session_id = "session-alpha".to_string();

    app.recent_projects = vec![path.clone()];
    app.recent_projects_edits = vec!["Alpha".to_string()];
    app.recent_projects_meta = vec![recent_meta(&path, Some("A"), Some("#336699"))];
    app.navigation_tabs.project_path = Some(path.clone());
    app.hovered_recent_project = Some(path.clone());
    app.project_sessions.insert(path.clone(), vec![session_info(&session_id, &path)]);

    let mut runtime = SessionRuntimeState::new();
    runtime.request_lifecycle.is_requesting = true;
    app.session_runtime_states.insert(session_id, runtime);

    let element: Element<'_, Message> = projects_list(&app, true);

    std::hint::black_box(element);
}


#[test]
fn projects_list_falls_back_to_current_sessions_for_attention() {
    let (mut app, _task) = App::new();
    let path = "/tmp/vibe-window-beta".to_string();
    let session_id = "session-beta".to_string();

    app.recent_projects = vec![path.clone()];
    app.recent_projects_edits = vec![String::new()];
    app.recent_projects_meta = vec![recent_meta(&path, None, Some("not-a-color"))];
    app.navigation_tabs.project_path = Some(path.clone());
    app.sessions = vec![session_info(&session_id, &path)];

    let mut runtime = SessionRuntimeState::new();
    runtime.has_unseen_success = true;
    app.session_runtime_states.insert(session_id, runtime);

    let element: Element<'_, Message> = projects_list(&app, false);

    std::hint::black_box(element);
}


#[test]
fn projects_list_builds_with_queue_attention() {
    let (mut app, _task) = App::new();
    let path = "/tmp/vibe-window-queue".to_string();
    let session_id = "session-queue".to_string();

    app.recent_projects = vec![path.clone()];
    app.recent_projects_edits = vec!["Queue Project".to_string()];
    app.navigation_tabs.project_path = Some(path.clone());
    app.project_sessions.insert(path.clone(), vec![session_info(&session_id, &path)]);

    let mut runtime = SessionRuntimeState::new();
    runtime.request_lifecycle.queue = vec![crate::app::state::QueueItem {
        created_ms: 1000,
        query: "test query".to_string(),
        attachments: vec![],
        root: None,
        model: None,
        reasoning_effort: None,
        acp_test: false,
        acp_model_source: Default::default(),
        acp_agent: None,
        acp_allowed_tools: None,
        agent: None,
        allowed_tools: None,
        require_agent_tool: false,
        acp_force_new_session: false,
        acp_history_mode: crate::app::state::AcpHistoryReplayMode::Discard,
        acp_recent_count: 3,
        full_access_enabled: true,
        send_behavior: crate::app::state::ChatSendBehavior::Queue,
        request_history_override: None,
        resume_history_only: false,
        workflow_mode_enabled: false,
        goal_mode_enabled: false,
    }];
    app.session_runtime_states.insert(session_id, runtime);

    let element: Element<'_, Message> = projects_list(&app, false);

    std::hint::black_box(element);
}


#[test]
fn projects_list_uses_empty_path_when_edit_has_no_matching_recent_project() {
    let (mut app, _task) = App::new();
    app.recent_projects.clear();
    app.recent_projects_edits = vec!["Loose edit".to_string()];
    app.recent_projects_meta.clear();

    let element: Element<'_, Message> = projects_list(&app, false);

    std::hint::black_box(element);
}


#[test]
fn projects_list_builds_without_attention_when_runtime_state_is_missing() {
    let (mut app, _task) = App::new();
    let path = "/tmp/no-runtime".to_string();
    app.recent_projects = vec![path.clone()];
    app.recent_projects_edits = vec!["No Runtime".to_string()];
    app.project_sessions.insert(path.clone(), vec![session_info("missing-runtime", &path)]);

    let element: Element<'_, Message> = projects_list(&app, false);

    std::hint::black_box(element);
}


#[test]
fn projects_list_does_not_use_current_sessions_for_unselected_project() {
    let (mut app, _task) = App::new();
    let listed_path = "/tmp/listed".to_string();
    app.recent_projects = vec![listed_path.clone()];
    app.recent_projects_edits = vec![String::new()];
    app.navigation_tabs.project_path = Some("/tmp/current".to_string());
    app.sessions = vec![session_info("current-session", "/tmp/current")];

    let mut runtime = SessionRuntimeState::new();
    runtime.request_lifecycle.is_requesting = true;
    app.session_runtime_states.insert("current-session".to_string(), runtime);

    let element: Element<'_, Message> = projects_list(&app, true);

    std::hint::black_box(element);
}


#[test]
fn projects_list_ignores_recent_project_meta_for_other_paths() {
    let (mut app, _task) = App::new();
    let listed_path = "/tmp/listed-without-meta".to_string();
    app.recent_projects = vec![listed_path.clone()];
    app.recent_projects_edits = vec!["Listed".to_string()];
    app.recent_projects_meta = vec![recent_meta("/tmp/other-project", Some("O"), Some("#112233"))];

    let element: Element<'_, Message> = projects_list(&app, false);

    std::hint::black_box(element);
}


#[cfg(not(target_arch = "wasm32"))]
#[test]
fn code_projects_rail_renders_titles_and_opens_the_four_column_grid() {
    use iced::advanced::{
        clipboard,
        renderer::{Headless, Style},
    };
    use iced::widget::row;
    use iced::{Event, Font, Point, Renderer, Size, mouse};
    use iced_runtime::user_interface::{Cache, UserInterface};

    let mut app = App::new().0;
    app.recent_projects = (0..7).map(|index| format!("/tmp/project-{index}")).collect();
    app.recent_projects_edits =
        (0..7).map(|index| format!("项目 {index} 的完整中文名称 Mixed Project Name")).collect();
    for font in crate::fonts::load_all() {
        iced::advanced::graphics::text::font_system().write().unwrap().load_font(font);
    }
    let mut renderer = iced::futures::executor::block_on(Renderer::new(
        Font::with_name("Noto Sans CJK SC"),
        14.0.into(),
        Some("tiny-skia"),
    ))
    .unwrap();
    let size = Size::new(640.0, 600.0);
    let rail = |app: &App| {
        row![container(projects_list(app, false)).width(56), Space::new().width(Length::Fill)]
    };
    let update = |ui: &mut UserInterface<'_, Message, Theme, Renderer>,
                  renderer: &mut Renderer,
                  point: Point,
                  events: &[Event]| {
        let mut messages = Vec::new();
        ui.update(
            events,
            mouse::Cursor::Available(point),
            renderer,
            &mut clipboard::Null,
            &mut messages,
        );
        messages
    };
    let click = [
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
    ];
    let move_to = |point| [Event::Mouse(mouse::Event::CursorMoved { position: point })];
    let menu_changed = |message: &Message, open| {
        matches!(
            message,
            Message::Project(message::ProjectMessage::CodeProjectsMenuChanged(value))
                if *value == open
        )
    };
    let trigger = Point::new(28.0, 327.0);
    let gap = Point::new(54.0, 327.0);
    let last_project = Point::new(331.0, 450.0);
    let outside = Point::new(560.0, 327.0);

    let seventh_name = app.recent_projects_edits.pop().unwrap();
    let mut six_projects = UserInterface::build(rail(&app), size, Cache::new(), &mut renderer);
    assert!(update(&mut six_projects, &mut renderer, trigger, &click).iter().any(|message| {
        matches!(message, Message::Project(message::ProjectMessage::OpenFolderPressed))
    }));
    drop(six_projects);
    app.recent_projects_edits.push(seventh_name);

    for theme in [Theme::Light, Theme::Dark] {
        app.navigation_mode.code_projects_menu_open = false;
        let mut ui = UserInterface::build(rail(&app), size, Cache::new(), &mut renderer);
        let style = Style { text_color: theme.palette().text };
        let first_project = Point::new(28.0, 26.0);
        update(&mut ui, &mut renderer, first_project, &move_to(first_project));
        ui.draw(&mut renderer, &theme, &style, mouse::Cursor::Available(first_project));
        let pixels = renderer.screenshot(Size::new(640, 600), 1.0, theme.palette().background);
        let background = &pixels[..4];
        let tooltip_pixel = (26 * 640 + 100) * 4;
        assert_ne!(&pixels[tooltip_pixel..tooltip_pixel + 4], background);
        assert!(
            update(&mut ui, &mut renderer, trigger, &move_to(trigger))
                .iter()
                .any(|message| menu_changed(message, true))
        );
        drop(ui);

        app.navigation_mode.code_projects_menu_open = true;
        let mut ui = UserInterface::build(rail(&app), size, Cache::new(), &mut renderer);
        for point in [gap, last_project] {
            assert!(
                !update(&mut ui, &mut renderer, point, &move_to(point))
                    .iter()
                    .any(|message| menu_changed(message, false))
            );
        }
        ui.draw(&mut renderer, &theme, &style, mouse::Cursor::Available(last_project));
        let pixels = renderer.screenshot(Size::new(640, 600), 1.0, theme.palette().background);
        let grid_pixel = (365 * 640 + 440) * 4;
        assert_ne!(&pixels[grid_pixel..grid_pixel + 4], &pixels[..4]);
        assert!(update(&mut ui, &mut renderer, last_project, &click).iter().any(|message| {
            matches!(
                message,
                Message::Project(message::ProjectMessage::OpenRecentPressed(path))
                    if path == "/tmp/project-6"
            )
        }));
        assert!(
            update(&mut ui, &mut renderer, outside, &move_to(outside))
                .iter()
                .any(|message| menu_changed(message, false))
        );
    }
}
