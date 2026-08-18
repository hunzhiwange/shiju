#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("settings_tests"));
}

#[test]
fn main_window_close_request_closes_task_pet_too() {
    let main_window_id = iced::window::Id::unique();
    let task_pet_window_id = iced::window::Id::unique();

    let windows = super::close_requested_windows(
        Some(main_window_id),
        Some(task_pet_window_id),
        main_window_id,
    );

    assert_eq!(windows.main_window_id, Some(main_window_id));
    assert_eq!(windows.task_pet_window_id, Some(task_pet_window_id));
}

#[test]
fn task_pet_close_request_only_closes_task_pet() {
    let main_window_id = iced::window::Id::unique();
    let task_pet_window_id = iced::window::Id::unique();

    let windows = super::close_requested_windows(
        Some(main_window_id),
        Some(task_pet_window_id),
        task_pet_window_id,
    );

    assert_eq!(windows.main_window_id, None);
    assert_eq!(windows.task_pet_window_id, Some(task_pet_window_id));
}

#[test]
fn unknown_window_close_request_closes_nothing() {
    let main_window_id = iced::window::Id::unique();
    let task_pet_window_id = iced::window::Id::unique();

    let windows = super::close_requested_windows(
        Some(main_window_id),
        Some(task_pet_window_id),
        iced::window::Id::unique(),
    );

    assert_eq!(windows.main_window_id, None);
    assert_eq!(windows.task_pet_window_id, None);
}

#[test]
fn shared_window_id_is_closed_only_once() {
    let shared_window_id = iced::window::Id::unique();

    let windows = super::close_requested_windows(
        Some(shared_window_id),
        Some(shared_window_id),
        shared_window_id,
    );

    assert_eq!(windows.main_window_id, Some(shared_window_id));
    assert_eq!(windows.task_pet_window_id, None);
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

    let _ = super::update(&mut app, ViewMessage::OpenSystemSettingsTab(SystemTab::General));

    assert!(app.show_system_settings);
    assert_eq!(app.system_settings_help_tab, None);
    assert_eq!(app.active_menu, None);
    assert!(!app.show_model_popover);
    assert!(!app.show_mode_popover);
    assert!(!app.show_file_popover);
    assert!(!app.show_usage_popover);
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

#[test]
fn project_file_settings_actions_toggle_panels_tabs_and_loading_state() {
    use crate::app::SettingsTab;
    use crate::app::message::view::ViewMessage;

    let mut app = crate::app::App::new().0;
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

    let initial_diff = app.show_diff;
    let _ = super::update(&mut app, ViewMessage::ToggleDiffPanel);
    assert_eq!(app.show_diff, !initial_diff);

    let initial_summary = app.show_git_diff_summary;
    let _ = super::update(&mut app, ViewMessage::ToggleGitDiffSummary);
    assert_eq!(app.show_git_diff_summary, !initial_summary);

    app.dragging_file_manager = true;
    app.file_manager_drag_anchor_x = Some(42.0);
    let _ = super::update(&mut app, ViewMessage::FileManagerPanelVisible(false));
    assert!(!app.show_file_manager);
    assert!(!app.dragging_file_manager);
    assert_eq!(app.file_manager_drag_anchor_x, None);

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
fn work_mode_ignores_code_only_terminal_and_review_toggles() {
    use crate::app::message::view::ViewMessage;
    use crate::app::state::WorkspaceMode;

    let mut app = crate::app::App::new().0;
    app.workspace_mode = WorkspaceMode::Work;
    app.show_diff = false;
    app.terminal.is_visible = false;

    let _ = super::update(&mut app, ViewMessage::ToggleDiffPanel);
    let _ = super::update(&mut app, ViewMessage::ToggleTerminalPanel);

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

    let _ = super::update(&mut app, ViewMessage::OpenAppUpdateModal);

    assert_eq!(app.active_menu, None);
    assert!(app.show_cli_install_modal);
    assert_eq!(app.cli_install_modal_title, "检测更新");
    assert_eq!(app.cli_install_modal_server_version, "未检测");
    assert!(app.cli_install_modal_show_update_action);
    assert!(!app.cli_install_modal_show_install_action);
    assert!(app.cli_install_modal_use_app_update_action);
    assert!(!app.cli_install_modal_is_checking_update);
    assert!(app.app_update_progress.is_none());
    assert!(app.app_update_progress_rx.is_none());
}

#[test]
fn install_and_update_requests_prepare_modal_state_before_async_work_runs() {
    use crate::app::message::view::{MenuType, ViewMessage};

    let mut app = crate::app::App::new().0;
    app.active_menu = Some(MenuType::Help);

    let _ = super::update(&mut app, ViewMessage::InstallCliTool);
    assert_eq!(app.active_menu, None);
    assert!(app.show_cli_install_modal);
    assert_eq!(app.cli_install_modal_title, "CLI 更新检测");
    assert_eq!(app.cli_install_modal_server_version, "未检测");
    assert!(app.cli_install_modal_show_update_action);
    assert!(app.cli_install_modal_show_install_action);
    assert!(!app.cli_install_modal_use_app_update_action);
    assert!(!app.cli_install_modal_is_checking_update);

    let _ = super::update(&mut app, ViewMessage::CheckCliToolUpdate);
    assert!(app.cli_install_modal_is_checking_update);
    assert!(app.cli_install_modal_message.contains("GitHub"));

    let _ = super::update(&mut app, ViewMessage::CheckAppUpdate);
    assert!(app.cli_install_modal_is_checking_update);
    assert!(!app.cli_install_modal_show_install_action);
    assert!(app.app_update_progress.is_none());
    assert!(app.app_update_progress_rx.is_none());

    let _ = super::update(&mut app, ViewMessage::RunAppUpdate);
    assert!(app.cli_install_modal_is_checking_update);
    assert!(!app.cli_install_modal_show_install_action);
    assert!(app.cli_install_modal_message.contains("正在下载"));
    assert!(app.app_update_progress_rx.is_some());

    let _ = super::update(&mut app, ViewMessage::RunInstallCliTool);
    assert!(!app.cli_install_modal_is_checking_update);
}

#[test]
fn closing_install_modal_hides_it_and_resets_update_actions() {
    use crate::app::message::view::ViewMessage;

    let mut app = crate::app::App::new().0;
    app.show_cli_install_modal = true;
    app.cli_install_modal_current_version = "old".into();
    app.cli_install_modal_server_version = "new".into();
    app.cli_install_modal_show_update_action = true;
    app.cli_install_modal_show_install_action = true;
    app.cli_install_modal_use_app_update_action = true;
    app.cli_install_modal_is_checking_update = true;

    let _ = super::update(&mut app, ViewMessage::CloseInstallCliModal);

    assert!(!app.show_cli_install_modal);
    assert!(app.cli_install_modal_current_version.is_empty());
    assert!(app.cli_install_modal_server_version.is_empty());
    assert!(!app.cli_install_modal_show_update_action);
    assert!(!app.cli_install_modal_show_install_action);
    assert!(!app.cli_install_modal_use_app_update_action);
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
fn cli_update_result_distinguishes_matching_new_and_failed_versions() {
    use crate::app::message::view::ViewMessage;

    let mut app = crate::app::App::new().0;
    app.cli_install_modal_current_version = "v1.2.3".into();
    app.cli_install_modal_is_checking_update = true;

    let _ = super::update(&mut app, ViewMessage::CheckCliToolUpdateFinished(Ok("v1.2.3".into())));
    assert!(!app.cli_install_modal_is_checking_update);
    assert_eq!(app.cli_install_modal_server_version, "v1.2.3");
    assert!(app.cli_install_modal_message.contains("已是最新版本"));

    let _ = super::update(&mut app, ViewMessage::CheckCliToolUpdateFinished(Ok("v2.0.0".into())));
    assert!(app.cli_install_modal_message.contains("发现新的 CLI 版本"));

    let _ = super::update(&mut app, ViewMessage::CheckCliToolUpdateFinished(Err("offline".into())));
    assert_eq!(app.cli_install_modal_server_version, "检测失败");
    assert_eq!(app.cli_install_modal_message, "offline");
}

#[test]
fn app_update_result_controls_install_action_for_each_outcome() {
    use crate::app::message::view::ViewMessage;

    let mut app = crate::app::App::new().0;
    app.cli_install_modal_current_version = "VibeWindow v1.2.3".into();

    let _ = super::update(&mut app, ViewMessage::CheckAppUpdateFinished(Ok("v1.2.3".into())));
    assert!(!app.cli_install_modal_show_install_action);
    assert!(app.cli_install_modal_message.contains("已是最新版本"));

    let _ = super::update(&mut app, ViewMessage::CheckAppUpdateFinished(Ok("v1.2.4".into())));
    assert!(app.cli_install_modal_show_install_action);
    assert!(app.cli_install_modal_message.contains("发现新版本"));

    let _ = super::update(
        &mut app,
        ViewMessage::CheckAppUpdateFinished(Err("server unavailable".into())),
    );
    assert!(!app.cli_install_modal_show_install_action);
    assert_eq!(app.cli_install_modal_server_version, "检测失败");
    assert_eq!(app.cli_install_modal_message, "server unavailable");
}

#[test]
fn app_update_progress_drains_available_values_and_drops_disconnected_receiver() {
    use crate::app::message::view::ViewMessage;

    let mut app = crate::app::App::new().0;
    let (sender, receiver) = std::sync::mpsc::channel();
    let first = vw_shared::update::UpdateProgress {
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

    let _ = super::update(&mut app, ViewMessage::AppUpdateFinished(Ok("updated".into())));
    assert!(!app.cli_install_modal_is_checking_update);
    assert!(app.app_update_progress_rx.is_none());
    assert_eq!(app.cli_install_modal_title, "应用更新完成");
    assert_eq!(app.cli_install_modal_message, "updated");

    let _ = super::update(&mut app, ViewMessage::AppUpdateFinished(Err("replace failed".into())));
    assert!(app.cli_install_modal_show_install_action);
    assert_eq!(app.cli_install_modal_title, "应用更新失败");
    assert_eq!(app.cli_install_modal_message, "replace failed");
}

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
fn normalize_version_uses_first_nonempty_line_and_last_token() {
    assert_eq!(super::normalize_version("\n  VibeWindow v1.2.3  \nignored"), "1.2.3");
    assert_eq!(super::normalize_version("v2.0.0"), "2.0.0");
    assert_eq!(super::normalize_version("  "), "");
}

#[test]
fn versions_match_ignores_prefix_and_product_name_but_not_version_changes() {
    assert!(super::versions_match("VibeWindow v1.2.3", "1.2.3"));
    assert!(super::versions_match("\nCLI v1.2.3\n", "v1.2.3"));
    assert!(!super::versions_match("v1.2.3", "v1.2.4"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn version_output_prefers_stdout_and_trims_first_nonempty_line() {
    let version = super::extract_version_output(
        b"\n  vibewindow v1.2.3  \nignored".to_vec(),
        b"stderr version".to_vec(),
    )
    .expect("stdout should contain a version");

    assert_eq!(version, "vibewindow v1.2.3");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn version_output_falls_back_to_stderr_and_rejects_blank_output() {
    assert_eq!(
        super::extract_version_output(Vec::new(), b"  vibewindow v2.0.0\n".to_vec())
            .expect("stderr should contain a version"),
        "vibewindow v2.0.0"
    );
    assert_eq!(
        super::extract_version_output(Vec::new(), b" \n\t".to_vec()).unwrap_err(),
        "CLI 未安装或未返回版本信息"
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn install_cli_binary_from_path_replaces_old_file_without_tail_bytes() {
    let dir = tempfile::tempdir().expect("temp dir should be created");
    let source_bin = dir.path().join("source-shiju");
    let target_bin = dir.path().join(if cfg!(windows) { "shiju.exe" } else { "shiju" });
    std::fs::write(&source_bin, b"new").expect("source binary should be written");
    std::fs::write(&target_bin, b"old-binary-tail").expect("old binary should be written");

    super::install_cli_binary_from_path(&source_bin, &target_bin)
        .expect("CLI binary should be installed");

    assert_eq!(std::fs::read(&target_bin).expect("target binary should be readable"), b"new");
    assert!(!super::staged_cli_binary_path(&target_bin).exists());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn staged_cli_path_is_hidden_and_stale_file_is_removed() {
    let dir = tempfile::tempdir().expect("temp dir should be created");
    let target = dir.path().join("shiju");
    let staged = super::staged_cli_binary_path(&target);

    assert_eq!(staged, dir.path().join(".shiju.installing"));
    std::fs::write(&staged, b"stale").expect("staged file should be written");
    super::remove_stale_staged_cli_binary(&staged).expect("staged file should be removed");
    assert!(!staged.exists());
    super::remove_stale_staged_cli_binary(&staged).expect("missing staged file is accepted");
}

#[cfg(all(unix, not(target_arch = "wasm32")))]
#[test]
fn executable_permission_is_set_on_staged_cli() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().expect("temp dir should be created");
    let staged = dir.path().join("staged-cli");
    std::fs::write(&staged, b"binary").expect("staged file should be written");
    std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o600))
        .expect("initial permissions should be set");

    super::ensure_cli_binary_executable(&staged).expect("permissions should be updated");

    let mode =
        std::fs::metadata(&staged).expect("metadata should be readable").permissions().mode();
    assert_eq!(mode & 0o777, 0o755);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn missing_profile_is_created_and_path_entry_is_idempotent() {
    let dir = tempfile::tempdir().expect("temp dir should be created");
    let profile = dir.path().join("nested").join("profile");
    let install_dir = dir.path().join("bin");
    assert!(!profile.exists());

    assert!(
        super::ensure_profile_contains_path(&profile, &install_dir)
            .expect("path entry should be appended")
    );
    let first = std::fs::read_to_string(&profile).expect("profile should be readable");
    assert!(first.ends_with('\n'));
    assert!(first.contains(install_dir.to_string_lossy().as_ref()));

    assert!(
        !super::ensure_profile_contains_path(&profile, &install_dir)
            .expect("existing path entry should be accepted")
    );
    assert_eq!(std::fs::read_to_string(&profile).expect("profile should remain readable"), first);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn existing_profile_without_trailing_newline_gets_a_separate_path_line() {
    let dir = tempfile::tempdir().expect("temp dir should be created");
    let profile = dir.path().join("profile");
    let install_dir = dir.path().join("bin");
    std::fs::write(&profile, "existing").expect("profile should be written");

    assert!(
        super::ensure_profile_contains_path(&profile, &install_dir)
            .expect("path entry should be appended")
    );

    let contents = std::fs::read_to_string(&profile).expect("profile should be readable");
    assert!(contents.starts_with("existing\n"));
    assert!(contents.ends_with('\n'));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn path_profiles_are_created_only_inside_supplied_home() {
    let home = tempfile::tempdir().expect("temporary home should be created");
    let install_dir = home.path().join(".vibewindow").join("bin");

    let updated = super::ensure_path_profiles(home.path(), &install_dir)
        .expect("isolated profiles should be updated");

    #[cfg(windows)]
    assert_eq!(updated.len(), 2);
    #[cfg(not(windows))]
    assert_eq!(updated.len(), 3);
    assert!(updated.iter().all(|path| path.starts_with(home.path().to_string_lossy().as_ref())));
    assert!(updated.iter().all(|path| std::path::Path::new(path).is_file()));
}
