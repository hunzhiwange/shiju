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

    assert_eq!(app.top_bar_gateway_tab, crate::app::state::TopBarGatewayTab::Plugins);
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

    assert_eq!(app.top_bar_gateway_tab, crate::app::state::TopBarGatewayTab::Mcp);
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
    assert_eq!(app.top_bar_gateway_tab, crate::app::state::TopBarGatewayTab::Mcp);
}

#[test]
fn system_settings_toggle_clears_help_and_flips_visibility() {
    use crate::app::components::system_settings::SystemTab;
    use crate::app::message::view::ViewMessage;

    let mut app = crate::app::App::new().0;
    app.show_system_settings = true;
    app.system_settings_help_tab = Some(SystemTab::General);

    let _ = super::update(&mut app, ViewMessage::ToggleSystemSettings);
    assert!(!app.show_system_settings);
    assert_eq!(app.system_settings_help_tab, None);

    let _ = super::update(&mut app, ViewMessage::ToggleSystemSettings);
    assert!(app.show_system_settings);
}

#[test]
fn opening_system_settings_tab_closes_transient_popovers() {
    use crate::app::components::system_settings::SystemTab;
    use crate::app::message::view::{MenuType, ViewMessage};

    let mut app = crate::app::App::new().0;
    app.show_system_settings = false;
    app.system_settings_help_tab = Some(SystemTab::Memory);
    app.active_menu = Some(MenuType::Help);
    app.show_model_popover = true;
    app.show_mode_popover = true;
    app.show_file_popover = true;
    app.show_usage_popover = true;
    app.show_session_tool_selector_popover = true;

    let _ = super::update(&mut app, ViewMessage::OpenSystemSettingsTab(SystemTab::General));

    assert!(app.show_system_settings);
    assert_eq!(app.system_settings_help_tab, None);
    assert_eq!(app.active_menu, None);
    assert!(!app.show_model_popover);
    assert!(!app.show_mode_popover);
    assert!(!app.show_file_popover);
    assert!(!app.show_usage_popover);
    assert!(!app.show_session_tool_selector_popover);
}

#[test]
fn opening_model_detail_selects_models_and_closes_transient_popovers() {
    use crate::app::components::system_settings::SystemTab;
    use crate::app::message::view::{MenuType, ViewMessage};
    use crate::app::state::ModelConfigTab;

    let mut app = crate::app::App::new().0;
    app.active_menu = Some(MenuType::GatewayServices);
    app.show_model_popover = true;
    app.show_mode_popover = true;
    app.show_file_popover = true;
    app.show_usage_popover = true;
    app.model_settings.active_tab = ModelConfigTab::QueryClassification;

    let _ = super::update(
        &mut app,
        ViewMessage::OpenSystemSettingsModelDetail("provider".into(), "model".into()),
    );

    assert!(app.show_system_settings);
    assert_eq!(app.system_settings_tab, SystemTab::Models);
    assert_eq!(app.model_settings.active_tab, ModelConfigTab::Models);
    assert_eq!(app.active_menu, None);
    assert!(!app.show_model_popover);
    assert!(!app.show_mode_popover);
    assert!(!app.show_file_popover);
    assert!(!app.show_usage_popover);
}

// 任务与结果：覆盖任务构造、调度或结果回写对公开状态的影响。
// 完成、失败与过期响应均按当前场景的既定契约处理。
#[test]
fn project_file_settings_actions_toggle_panels_tabs_and_loading_state() {
    use crate::app::SettingsTab;
    use crate::app::message::view::ViewMessage;

    let mut app = crate::app::App::new().0;
    app.navigation_mode.workspace_mode = crate::app::state::WorkspaceMode::Code;
    app.project_path = Some("/tmp/project".to_string());
    app.show_settings = false;

    let _ = super::update(&mut app, ViewMessage::ToggleSettingsPanel);
    assert!(app.show_settings);
    assert!(app.project_sessions_loading.contains("/tmp/project"));

    let _ = super::update(&mut app, ViewMessage::ToggleSettingsPanel);
    assert!(!app.show_settings);

    let _ = super::update(&mut app, ViewMessage::ProjectFileShowSessions);
    assert!(app.show_settings);
    assert_eq!(app.settings_tab, SettingsTab::Sessions);

    let _ = super::update(&mut app, ViewMessage::ProjectFileShowProjects);
    assert!(app.show_settings);
    assert_eq!(app.settings_tab, SettingsTab::Projects);

    app.show_preview_context_menu = true;
    let _ = super::update(&mut app, ViewMessage::ProjectFileSaveAll);
    assert!(!app.show_preview_context_menu);
}

#[test]
fn panel_visibility_and_menu_actions_update_only_local_state() {
    use crate::app::message::view::{MenuType, ViewMessage};

    let mut app = crate::app::App::new().0;
    app.navigation_mode.workspace_mode = crate::app::state::WorkspaceMode::Code;

    let initial_diff = app.show_diff;
    let _ = super::update(&mut app, ViewMessage::ToggleDiffPanel);
    assert_eq!(app.show_diff, !initial_diff);

    let initial_summary = app.git.show_git_diff_summary;
    let _ = super::update(&mut app, ViewMessage::ToggleGitDiffSummary);
    assert_eq!(app.git.show_git_diff_summary, !initial_summary);

    app.layout_drag.dragging_file_manager = true;
    app.layout_drag.file_manager_drag_anchor_x = Some(42.0);
    let _ = super::update(&mut app, ViewMessage::FileManagerPanelVisible(false));
    assert!(!app.file_manager.show_file_manager);
    assert!(!app.layout_drag.dragging_file_manager);
    assert_eq!(app.layout_drag.file_manager_drag_anchor_x, None);

    let _ = super::update(&mut app, ViewMessage::AutoMaxToggled(true));
    assert!(app.auto_max_mode);

    let _ = super::update(&mut app, ViewMessage::ToggleMenu(Some(MenuType::File)));
    assert_eq!(app.active_menu, Some(MenuType::File));
    let _ = super::update(&mut app, ViewMessage::ToggleMenu(Some(MenuType::File)));
    assert_eq!(app.active_menu, None);
    let _ = super::update(&mut app, ViewMessage::ToggleMenu(Some(MenuType::Help)));
    assert_eq!(app.active_menu, Some(MenuType::Help));
    let _ = super::update(&mut app, ViewMessage::ToggleMenu(None));
    assert_eq!(app.active_menu, None);

    app.active_menu = Some(MenuType::View);
    let _ = super::update(&mut app, ViewMessage::MenuAction(Box::new(crate::app::Message::None)));
    assert_eq!(app.active_menu, None);
}

#[test]
fn work_mode_ignores_code_only_review_toggle() {
    use crate::app::message::view::ViewMessage;
    use crate::app::state::WorkspaceMode;

    let mut app = crate::app::App::new().0;
    app.navigation_mode.workspace_mode = WorkspaceMode::Work;
    app.show_diff = false;
    app.terminal.is_visible = false;

    let _ = super::update(&mut app, ViewMessage::ToggleDiffPanel);

    assert!(!app.show_diff);
    assert!(!app.terminal.is_visible);
}

#[test]
fn about_modal_toggle_round_trips_visibility() {
    use crate::app::message::view::ViewMessage;

    let mut app = crate::app::App::new().0;
    let initial = app.show_about_modal;

    let _ = super::update(&mut app, ViewMessage::ToggleAboutModal);
    assert_eq!(app.show_about_modal, !initial);

    let _ = super::update(&mut app, ViewMessage::ToggleAboutModal);
    assert_eq!(app.show_about_modal, initial);
}

#[test]
fn app_update_modal_initializes_application_update_state() {
    use crate::app::message::view::{MenuType, ViewMessage};

    let mut app = crate::app::App::new().0;
    app.active_menu = Some(MenuType::Help);
    app.cli_install_modal_server_version = "stale".into();
    app.cli_install_modal_show_install_action = true;
    app.cli_install_modal_use_app_update_action = false;
    app.cli_install_modal_is_checking_update = true;

    let task = super::update(&mut app, ViewMessage::OpenAppUpdateModal);

    assert_eq!(app.active_menu, None);
    assert!(app.show_cli_install_modal);
    assert_eq!(app.cli_install_modal_title, "检测更新");
    assert_eq!(app.cli_install_modal_server_version, "未检测");
    assert!(app.cli_install_modal_show_update_action);
    assert!(!app.cli_install_modal_show_install_action);
    assert!(app.cli_install_modal_use_app_update_action);
    assert!(app.cli_install_modal_is_checking_update);
    assert!(app.desktop_update.checking);
    assert!(task.units() > 0);
    assert!(app.app_update_progress.is_none());
    assert!(app.app_update_progress_rx.is_none());

    let request = app.desktop_update.request;
    let duplicate = super::update(&mut app, ViewMessage::OpenAppUpdateModal);
    assert_eq!(duplicate.units(), 0);
    assert_eq!(app.desktop_update.request, request);
    let source = app.desktop_update.source.unwrap();
    let _ = super::update(
        &mut app,
        ViewMessage::CheckAppUpdateFinished(request, source, Err("offline".into())),
    );
    let retry = super::update(&mut app, ViewMessage::CheckAppUpdate);
    assert!(retry.units() > 0);
    assert!(app.desktop_update.checking);
}

#[test]
fn install_and_update_requests_prepare_modal_state_before_async_work_runs() {
    use crate::app::message::view::{MenuType, ViewMessage};

    let mut app = crate::app::App::new().0;
    app.active_menu = Some(MenuType::Help);

    let task = super::update(&mut app, ViewMessage::InstallCliTool);
    assert_eq!(app.active_menu, None);
    assert!(app.show_cli_install_modal);
    assert_eq!(app.cli_install_modal_title, "CLI 更新检测");
    assert_eq!(app.cli_install_modal_server_version, "未检测");
    assert!(app.cli_install_modal_show_update_action);
    assert!(app.cli_install_modal_show_install_action);
    assert!(!app.cli_install_modal_use_app_update_action);
    assert!(app.cli_install_modal_is_checking_update);
    assert!(app.desktop_update.checking);
    assert!(task.units() > 0);

    let request = app.desktop_update.request;
    let duplicate = super::update(&mut app, ViewMessage::CheckCliToolUpdate);
    assert_eq!(duplicate.units(), 0);
    assert_eq!(app.desktop_update.request, request);
    let source = app.desktop_update.source.unwrap();
    let _ = super::update(
        &mut app,
        ViewMessage::CheckCliToolUpdateFinished(request, source, Err("offline".into())),
    );
    let task = super::update(&mut app, ViewMessage::CheckCliToolUpdate);
    assert!(task.units() > 0);
    assert!(app.cli_install_modal_is_checking_update);
    assert!(app.cli_install_modal_message.contains("最新 CLI 版本"));

    let request = app.desktop_update.request;
    let _ = super::update(
        &mut app,
        ViewMessage::CheckCliToolUpdateFinished(request, source, Err("offline".into())),
    );
    let _ = super::update(&mut app, ViewMessage::CheckAppUpdate);
    assert!(app.cli_install_modal_is_checking_update);
    assert!(!app.cli_install_modal_show_install_action);
    assert!(app.app_update_progress.is_none());
    assert!(app.app_update_progress_rx.is_none());

    let _ = super::update(&mut app, ViewMessage::RunAppUpdate);
    assert!(app.cli_install_modal_is_checking_update);
    assert!(!app.cli_install_modal_show_install_action);
    assert!(app.app_update_progress_rx.is_none()); // No checked snapshot: no download.

    let _ = super::update(&mut app, ViewMessage::RunInstallCliTool);
    assert!(!app.cli_install_modal_is_checking_update);
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
        assert_eq!(app.cli_install_modal_show_install_action, installable);
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
    assert!(!app.cli_install_modal_show_install_action);
    assert_eq!(app.cli_install_modal_message, "server unavailable");
    assert!(app.active_toast.is_some());
}

#[test]
fn app_update_progress_drains_available_values_and_drops_disconnected_receiver() {
    use crate::app::message::view::ViewMessage;

    let mut app = crate::app::App::new().0;
    let (sender, receiver) = std::sync::mpsc::channel();
    let first = update_types::UpdateProgress {
        asset_name: "app".into(),
        download_url: "https://example.invalid/first".into(),
        downloaded_bytes: 10,
        total_bytes: Some(100),
        bytes_per_second: 2.0,
        average_bytes_per_second: 1.0,
    };
    let mut latest = first.clone();
    latest.downloaded_bytes = 20;
    sender.send(first).expect("first progress should be queued");
    sender.send(latest.clone()).expect("latest progress should be queued");
    drop(sender);
    app.app_update_progress_rx = Some(receiver);

    let _ = super::update(&mut app, ViewMessage::AppUpdateProgressTick);

    assert_eq!(app.app_update_progress, Some(latest));
    assert!(app.app_update_progress_rx.is_none());
}

#[test]
fn app_update_finish_clears_receiver_and_sets_success_or_failure_state() {
    use crate::app::message::view::ViewMessage;

    let mut app = crate::app::App::new().0;
    let (_sender, receiver) = std::sync::mpsc::channel();
    app.app_update_progress_rx = Some(receiver);
    app.cli_install_modal_is_checking_update = true;

    app.desktop_update.busy = true;
    let _ = super::update(
        &mut app,
        ViewMessage::AppUpdateFinished(Ok("安装器已打开，请完成安装".into())),
    );
    assert!(!app.cli_install_modal_is_checking_update);
    assert!(app.app_update_progress_rx.is_none());
    assert_eq!(app.cli_install_modal_title, "安装包已就绪");
    assert_eq!(app.cli_install_modal_message, "安装器已打开，请完成安装");

    app.desktop_update.busy = true;
    let _ = super::update(&mut app, ViewMessage::AppUpdateFinished(Err("replace failed".into())));
    assert!(app.active_toast.is_some());
    assert_eq!(app.cli_install_modal_message, "replace failed");
}

// 任务与结果：覆盖任务构造、调度或结果回写对公开状态的影响。
// 完成、失败与过期响应均按当前场景的既定契约处理。
#[test]
fn install_result_resets_update_state_and_sets_outcome_copy() {
    use crate::app::message::view::ViewMessage;

    let mut app = crate::app::App::new().0;
    app.cli_install_modal_current_version = "old".into();
    app.cli_install_modal_server_version = "new".into();
    app.cli_install_modal_show_update_action = true;
    app.cli_install_modal_show_install_action = true;
    app.cli_install_modal_use_app_update_action = true;
    app.cli_install_modal_is_checking_update = true;

    let _ = super::update(&mut app, ViewMessage::InstallCliToolFinished(Ok("installed".into())));

    assert!(app.show_cli_install_modal);
    assert_eq!(app.cli_install_modal_title, "CLI 安装完成");
    assert_eq!(app.cli_install_modal_message, "installed");
    assert!(app.cli_install_modal_current_version.is_empty());
    assert!(app.cli_install_modal_server_version.is_empty());
    assert!(!app.cli_install_modal_show_update_action);
    assert!(!app.cli_install_modal_show_install_action);
    assert!(!app.cli_install_modal_use_app_update_action);
    assert!(!app.cli_install_modal_is_checking_update);

    let _ = super::update(
        &mut app,
        ViewMessage::InstallCliToolFinished(Err("permission denied".into())),
    );
    assert_eq!(app.cli_install_modal_title, "CLI 安装失败");
    assert_eq!(app.cli_install_modal_message, "permission denied");
}

#[test]
fn menu_hover_only_switches_an_already_open_menu() {
    use crate::app::message::view::{MenuType, ViewMessage};

    let mut app = crate::app::App::new().0;
    let _ = super::update(&mut app, ViewMessage::MenuHovered(MenuType::Help));
    assert_eq!(app.active_menu, None);

    app.active_menu = Some(MenuType::File);
    let _ = super::update(&mut app, ViewMessage::MenuHovered(MenuType::View));
    assert_eq!(app.active_menu, Some(MenuType::View));
}

#[test]
fn app_update_download_survives_close_and_blocks_duplicate_and_source_change() {
    use crate::app::message::ViewMessage;
    use desktop_release::{DesktopRelease, UpdateSource};
    let mut app = crate::app::App::new().0;
    let _ = super::update(&mut app, ViewMessage::OpenAppUpdateModal);
    app.desktop_update.checking = false;
    app.desktop_update.release = Some(DesktopRelease {
        source: UpdateSource::Shiju,
        version: "99.0.0".into(),
        assets: vec![],
    });
    let snapshot = app.desktop_update.release.clone();
    let _ = super::update(&mut app, ViewMessage::RunAppUpdate);
    assert!(app.desktop_update.busy);
    assert!(app.app_update_progress_rx.is_some());
    let _ = super::update(&mut app, ViewMessage::CloseInstallCliModal);
    let _ = super::update(&mut app, ViewMessage::OpenAppUpdateModal);
    let duplicate = super::update(&mut app, ViewMessage::RunAppUpdate);
    assert!(duplicate.units() == 0);
    let _ = super::update(&mut app, ViewMessage::AppUpdateSourceSelected(UpdateSource::Github));
    let _ = super::update(&mut app, ViewMessage::CheckAppUpdate);
    assert_eq!(app.desktop_update.release, snapshot);
    assert_eq!(app.desktop_update.source, Some(UpdateSource::Shiju));
    assert!(app.app_update_progress_rx.is_some());
}

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

#[tokio::test(flavor = "multi_thread")]
async fn app_update_download_consumes_checked_snapshot_without_refetching_latest() {
    use crate::app::message::ViewMessage;
    use iced_runtime::futures::futures::StreamExt;
    use desktop_release::{DesktopRelease, UpdateSource};
    let mut app = crate::app::App::new().0;
    app.desktop_update.source = Some(UpdateSource::Github);
    // A deliberately incomplete snapshot must fail its own asset validation before network I/O.
    app.desktop_update.release = Some(DesktopRelease {
        source: UpdateSource::Github,
        version: "99.0.0".into(),
        assets: vec![],
    });
    let task = super::update(&mut app, ViewMessage::RunAppUpdate);
    let mut stream = iced_runtime::task::into_stream(task).unwrap();
    let action = tokio::time::timeout(std::time::Duration::from_secs(5), stream.next())
        .await
        .unwrap()
        .unwrap();
    match action {
        iced_runtime::Action::Output(crate::app::Message::View(
            ViewMessage::AppUpdateDownloadPrepared(_, Err(error)),
        )) => {
            assert!(error.contains("exactly six"), "{error}");
        }
        _ => panic!("expected snapshot validation failure"),
    }
}

#[test]
fn terminal_modes_toggle_preserves_session_and_height_across_mode_roundtrip() {
    use crate::app::message::ViewMessage;
    use crate::app::state::WorkspaceMode;
    let tmp = tempfile::tempdir().expect("temporary project");
    let mut app = crate::app::App::new().0;
    app.project_path = Some(tmp.path().to_string_lossy().into_owned());
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
