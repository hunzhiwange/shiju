// 宿主保留关闭清理、配置持久化和更新任务投影；窗口关闭请求分类已迁入 desktop-navigation。
// 验证窗口关闭请求、关停幂等性与设置标签刷新。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

// 编译接线：引用父模块符号，确保测试片段仍由真实模块包含。
// 该冒烟用例只承担接线职责，具体行为由同领域测试覆盖。
#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("settings_tests"));
}

#[test]
fn main_window_close_starts_shutdown_once() {
    let mut app = crate::app::App::new().0;

    assert!(super::begin_shutdown(&mut app.shutdown_requested));
    assert!(app.shutdown_requested);
    assert!(!super::begin_shutdown(&mut app.shutdown_requested));
}

#[test]
fn gateway_plugins_tab_selection_starts_plugin_refresh() {
    let mut app = crate::app::App::new().0;

    let _ = super::update(
        &mut app,
        crate::app::message::view::ViewMessage::GatewayServicesTabSelected(
            crate::app::state::TopBarGatewayTab::Plugins,
        ),
    );

    assert_eq!(app.navigation_panels.top_bar_gateway_tab, crate::app::state::TopBarGatewayTab::Plugins);
    assert!(app.skills_settings.plugin_loading);
}

#[test]
fn gateway_mcp_tab_selection_starts_mcp_refresh() {
    let mut app = crate::app::App::new().0;

    let _ = super::update(
        &mut app,
        crate::app::message::view::ViewMessage::GatewayServicesTabSelected(
            crate::app::state::TopBarGatewayTab::Mcp,
        ),
    );

    assert_eq!(app.navigation_panels.top_bar_gateway_tab, crate::app::state::TopBarGatewayTab::Mcp);
    assert!(app.mcp_settings.loading);
}

#[test]
fn gateway_refresh_is_not_restarted_while_loading() {
    let mut app = crate::app::App::new().0;
    app.skills_settings.plugin_loading = true;
    app.mcp_settings.loading = true;

    let _ = super::update(
        &mut app,
        crate::app::message::view::ViewMessage::GatewayServicesTabSelected(
            crate::app::state::TopBarGatewayTab::Plugins,
        ),
    );
    let _ = super::update(
        &mut app,
        crate::app::message::view::ViewMessage::GatewayServicesTabSelected(
            crate::app::state::TopBarGatewayTab::Mcp,
        ),
    );

    assert!(app.skills_settings.plugin_loading);
    assert!(app.mcp_settings.loading);
    assert_eq!(app.navigation_panels.top_bar_gateway_tab, crate::app::state::TopBarGatewayTab::Mcp);
}

#[test]
fn opening_model_detail_selects_models_and_closes_transient_popovers() {
    use crate::app::components::system_settings::SystemTab;
    use crate::app::message::view::{MenuType, ViewMessage};
    use crate::app::state::ModelConfigTab;

    let mut app = crate::app::App::new().0;
    app.navigation_panels.active_menu = Some(MenuType::GatewayServices);
    app.popover_visibility.model = true;
    app.popover_visibility.mode = true;
    app.popover_visibility.file = true;
    app.popover_visibility.usage = true;
    app.model_settings.active_tab = ModelConfigTab::QueryClassification;

    let _ = super::update(
        &mut app,
        ViewMessage::OpenSystemSettingsModelDetail("provider".into(), "model".into()),
    );

    assert!(app.navigation_panels.show_system_settings);
    assert_eq!(app.navigation_panels.system_settings_tab, SystemTab::Models);
    assert_eq!(app.model_settings.active_tab, ModelConfigTab::Models);
    assert_eq!(app.navigation_panels.active_menu, None);
    assert!(!app.popover_visibility.model);
    assert!(!app.popover_visibility.mode);
    assert!(!app.popover_visibility.file);
    assert!(!app.popover_visibility.usage);
}

// 任务与结果：覆盖任务构造、调度或结果回写对公开状态的影响。
// 完成、失败与过期响应均按当前场景的既定契约处理。
#[test]
fn project_file_settings_actions_toggle_panels_tabs_and_loading_state() {
    use crate::app::SettingsTab;
    use crate::app::message::view::ViewMessage;

    let mut app = crate::app::App::new().0;
    app.navigation_mode.workspace_mode = crate::app::state::WorkspaceMode::Code;
    app.navigation_tabs.project_path = Some("/tmp/project".to_string());
    app.navigation_panels.show_settings = false;

    let _ = super::update(&mut app, ViewMessage::ToggleSettingsPanel);
    assert!(app.navigation_panels.show_settings);
    assert!(app.project_sessions_loading.contains("/tmp/project"));

    let _ = super::update(&mut app, ViewMessage::ToggleSettingsPanel);
    assert!(!app.navigation_panels.show_settings);

    let _ = super::update(&mut app, ViewMessage::ProjectFileShowSessions);
    assert!(app.navigation_panels.show_settings);
    assert_eq!(app.navigation_panels.settings_tab, SettingsTab::Sessions);

    let _ = super::update(&mut app, ViewMessage::ProjectFileShowProjects);
    assert!(app.navigation_panels.show_settings);
    assert_eq!(app.navigation_panels.settings_tab, SettingsTab::Projects);

    app.show_preview_context_menu = true;
    let _ = super::update(&mut app, ViewMessage::ProjectFileSaveAll);
    assert!(!app.show_preview_context_menu);
}

#[test]
fn restart_failure_is_exposed_without_exiting() {
    use crate::app::message::view::ViewMessage;

    let mut app = crate::app::App::new().0;

    let _ = super::update(&mut app, ViewMessage::RestartAppFinished(Err("restart denied".into())));

    assert_eq!(app.error_message.as_deref(), Some("restart denied"));
}

#[test]
fn app_update_result_controls_install_action_for_each_outcome() {
    use crate::app::message::ViewMessage;
    use desktop_release::{DesktopRelease, UpdateSource};
    let mut app = crate::app::App::new().0;
    for (version, installable) in
        [(env!("CARGO_PKG_VERSION"), false), ("0.0.0", false), ("99.0.0", true)]
    {
        let _ = super::update(&mut app, ViewMessage::CheckAppUpdate);
        let request = app.desktop_update.request;
        let release =
            DesktopRelease { source: UpdateSource::Shiju, version: version.into(), assets: vec![] };
        let _ = super::update(
            &mut app,
            ViewMessage::CheckAppUpdateFinished(request, UpdateSource::Shiju, Ok(release.clone())),
        );
        assert_eq!(app.update_panel.cli_install_modal_show_install_action, installable);
        assert_eq!(app.desktop_update.release, Some(release));
    }
    let _ = super::update(&mut app, ViewMessage::CheckAppUpdate);
    let request = app.desktop_update.request;
    let _ = super::update(
        &mut app,
        ViewMessage::CheckAppUpdateFinished(
            request,
            UpdateSource::Shiju,
            Err("server unavailable".into()),
        ),
    );
    assert!(!app.update_panel.cli_install_modal_show_install_action);
    assert_eq!(app.update_panel.cli_install_modal_message, "server unavailable");
    assert!(app.active_toast.is_some());
}

#[test]
fn app_update_finish_clears_receiver_and_sets_success_or_failure_state() {
    use crate::app::message::view::ViewMessage;

    let mut app = crate::app::App::new().0;
    let (_sender, receiver) = std::sync::mpsc::channel();
    app.update_panel.app_update_progress_rx = Some(receiver);
    app.update_panel.cli_install_modal_is_checking_update = true;

    app.desktop_update.busy = true;
    let _ = super::update(
        &mut app,
        ViewMessage::AppUpdateFinished(Ok("安装器已打开，请完成安装".into())),
    );
    assert!(!app.update_panel.cli_install_modal_is_checking_update);
    assert!(app.update_panel.app_update_progress_rx.is_none());
    assert_eq!(app.update_panel.cli_install_modal_title, "安装包已就绪");
    assert_eq!(app.update_panel.cli_install_modal_message, "安装器已打开，请完成安装");

    app.desktop_update.busy = true;
    let _ = super::update(&mut app, ViewMessage::AppUpdateFinished(Err("replace failed".into())));
    assert!(app.active_toast.is_some());
    assert_eq!(app.update_panel.cli_install_modal_message, "replace failed");
}

// 任务与结果：覆盖任务构造、调度或结果回写对公开状态的影响。
// 完成、失败与过期响应均按当前场景的既定契约处理。

#[test]
fn app_update_save_failure_keeps_selection_and_shows_unsaved_toast() {
    use crate::app::message::ViewMessage;
    use desktop_release::UpdateSource;
    let mut app = crate::app::App::new().0;
    let _ = crate::app::message::view::update(
        &mut app,
        ViewMessage::AppUpdateSourceSelected(UpdateSource::Github),
    );
    let _ = crate::app::message::view::update(
        &mut app,
        ViewMessage::AppUpdateSourceSaved(Err("offline".into())),
    );
    assert_eq!(app.desktop_update.source, Some(UpdateSource::Github));
    assert!(!app.desktop_update.saving);
    assert!(app.active_toast.as_ref().unwrap().message.contains("未保存"));
}

#[test]
fn terminal_modes_toggle_preserves_session_and_height_across_mode_roundtrip() {
    use crate::app::message::ViewMessage;
    use crate::app::state::WorkspaceMode;
    let tmp = tempfile::tempdir().expect("temporary project");
    let mut app = crate::app::App::new().0;
    app.navigation_tabs.project_path = Some(tmp.path().to_string_lossy().into_owned());
    app.terminal.is_visible = false;
    app.terminal.height = 260.0;
    let _ = crate::app::message::view::update(
        &mut app,
        ViewMessage::WorkspaceModeSelected(WorkspaceMode::Work),
    );
    let _ = super::update(&mut app, ViewMessage::ToggleTerminalPanel);
    assert!(app.terminal.is_visible);
    assert_eq!(app.terminal.tabs.len(), 1);
    let id = app.terminal.tabs[0].id;
    for mode in [WorkspaceMode::Code, WorkspaceMode::Work] {
        let _ =
            crate::app::message::view::update(&mut app, ViewMessage::WorkspaceModeSelected(mode));
        let _ = super::update(&mut app, ViewMessage::ToggleTerminalPanel);
        assert!(!app.terminal.is_visible);
        assert_eq!(app.terminal.tabs[0].id, id);
        let _ = super::update(&mut app, ViewMessage::ToggleTerminalPanel);
        assert!(app.terminal.is_visible);
        assert_eq!(app.terminal.tabs.len(), 1);
        assert_eq!(app.terminal.active_id, Some(id));
        assert_eq!(app.terminal.height, 260.0);
    }
}
