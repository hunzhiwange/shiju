//! Navigation-owned initial states; the host only assembles domain states and native resources.
use std::collections::{HashMap, HashSet};

pub mod external_apps;

fn file_manager_width(cfg: &serde_json::Value) -> f32 {
    cfg.get("file_manager_width")
        .and_then(serde_json::Value::as_f64)
        .map(|value| value as f32)
        .unwrap_or(260.0)
        .clamp(180.0, 600.0)
}

pub fn work_workspace() -> crate::panel::work_workspace::State {
    crate::panel::work_workspace::State {
        work_resource_mode: crate::presentation::WorkResourceMode::default(),
        work_resources_visible: false,
        dragging_work_split: false,
        work_split_drag_anchor_x: None,
        work_resource_fullscreen: false,
        work_resource_half_fullscreen: false,
        work_split_ratio: 0.6,
        work_chat_fullscreen: false,
        work_chat_half_fullscreen: false,
        work_expanded_spaces: HashSet::new(),
    }
}

pub fn workspace_mode(cfg: &serde_json::Value) -> crate::panel::workspace_mode::State {
    crate::panel::workspace_mode::State {
        workspace_mode: crate::components::top_bar::types::WorkspaceMode::from_config(
            cfg.get("workspace_mode").and_then(serde_json::Value::as_str),
        ),
        work_current_project_menu: None,
        show_workspace_mode_popover: false,
        code_projects_menu_open: false,
    }
}

pub fn layout_drag(cfg: &serde_json::Value) -> crate::view_layout::drag::State {
    crate::view_layout::drag::State {
        dragging_file_manager: false,
        dragging_settings: false,
        dragging_split: false,
        dragging_work_sidebar: false,
        file_manager_drag_anchor_x: None,
        file_manager_start_width: file_manager_width(cfg),
        file_manager_width: file_manager_width(cfg),
        settings_drag_anchor_x: None,
        settings_drag_start_width: 0.0,
        settings_panel_width: 476.0,
        split_drag_anchor_x: None,
        split_drag_start_ratio: 0.0,
        split_ratio: 0.6,
        work_sidebar_drag_anchor_x: None,
        work_sidebar_drag_start_width: 292.0,
        work_sidebar_width: 292.0,
        work_split_drag_start_ratio: 0.6,
    }
}

pub fn panels(cfg: &serde_json::Value) -> crate::panel::settings::State {
    crate::panel::settings::State {
        show_settings: cfg
            .get("show_settings")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true),
        settings_tab: crate::ui_types::SettingsTab::Sessions,
        show_system_settings: false,
        system_settings_tab: desktop_ui_controls::settings_navigation::SystemTab::General,
        system_settings_help_tab: None,
        active_menu: None,
        show_about_modal: false,
        auto_max_mode: cfg
            .get("auto_max_mode")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        show_diff: true,
        top_bar_gateway_tab: crate::components::top_bar::types::TopBarGatewayTab::default(),
        cli_commands_tab: Default::default(),
    }
}

pub fn tabs() -> crate::panel::tabs::State {
    crate::panel::tabs::State {
        apps_search_query: String::new(),
        project_path: None,
        project_preview_tabs: HashMap::new(),
        preview_tabs: Vec::new(),
        project_preview_active_path: HashMap::new(),
        active_preview_path: None,
        screen: crate::ui_types::Screen::Home,
        active_tab_id: Some("home".to_string()),
        preview_tab_menu_path: None,
        preview_tab_menu_pos: None,
        home_apps_bar_scroll_x: 0.0,
        home_apps_bar_scroll_id: iced::widget::Id::new("home_apps_bar"),
        open_tabs: vec![crate::components::top_bar::types::AppTab {
            id: "home".to_string(),
            title: "首页".to_string(),
            screen: crate::ui_types::Screen::Home,
            project_path: None,
        }],
        hovered_tab_id: None,
    }
}

pub fn sidebar(cfg: &serde_json::Value) -> crate::panel::work_sidebar::State {
    crate::panel::work_sidebar::State {
        work_sidebar_visible_tasks: crate::presentation::WORK_SIDEBAR_INITIAL_VISIBLE_ITEMS,
        work_sidebar_visible_spaces: crate::presentation::WORK_SIDEBAR_INITIAL_VISIBLE_ITEMS,
        work_sidebar_space_task_visible: HashMap::new(),
        work_sidebar_project_nav_hovered: false,
        work_sidebar_filter_open: false,
        work_sidebar_more_open: false,
        work_sidebar_collapsed: cfg
            .get("work_sidebar_collapsed")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        work_sidebar_filter_scope: crate::presentation::WorkSidebarFilterScope::default(),
        work_sidebar_filter_status: crate::presentation::WorkSidebarFilterStatus::default(),
        work_sidebar_filter_time: crate::presentation::WorkSidebarFilterTime::default(),
        work_sidebar_tab: crate::presentation::WorkSidebarTab::default(),
        work_space_picker_open: false,
    }
}

/// Apply only navigation-owned preferences; omitted fields preserve in-flight local state.
pub fn apply_preferences(
    navigation_panels: &mut crate::panel::settings::State,
    navigation_mode: &mut crate::panel::workspace_mode::State,
    navigation_sidebar: &mut crate::panel::work_sidebar::State,
    external_navigation: &mut crate::panel::external::State,
    layout_drag: &mut crate::view_layout::drag::State,
    cfg: &serde_json::Value,
) {
    navigation_panels.auto_max_mode = cfg
        .get("auto_max_mode")
        .and_then(|v| v.as_bool())
        .unwrap_or(navigation_panels.auto_max_mode);
    if let Some(workspace_mode) = cfg
        .get("workspace_mode")
        .and_then(|value| value.as_str())
        .and_then(crate::components::top_bar::types::WorkspaceMode::parse)
    {
        navigation_mode.workspace_mode = workspace_mode;
    }
    if let Some(collapsed) = cfg.get("work_sidebar_collapsed").and_then(|v| v.as_bool()) {
        navigation_sidebar.work_sidebar_collapsed = collapsed;
    }
    navigation_panels.show_settings = cfg
        .get("show_settings")
        .and_then(|v| v.as_bool())
        .unwrap_or(navigation_panels.show_settings);
    if let Some(target) = cfg
        .get("open_external_app")
        .and_then(|v| v.as_str())
        .and_then(crate::external_apps_update::parse_external_open_app)
    {
        external_navigation.open_external_app = target;
    }
    if !layout_drag.dragging_file_manager
        && let Some(width) = cfg
            .get("file_manager_width")
            .and_then(|v| v.as_f64())
            .map(|v| (v as f32).clamp(180.0, 600.0))
    {
        layout_drag.file_manager_width = width;
        layout_drag.file_manager_start_width = width;
    }
}
