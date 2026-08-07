use super::*;
use crate::app::skill_hub::{
    InstalledSkill, LoadState, ShijuPackageRefreshState, ShijuPackageStatus,
};
use crate::app::state::WorkspaceMode;
use crate::app::{App, Message, message};
use iced::advanced::widget::Tree;
use vw_config_types::ui::{
    AppSystemSettingsConfig, QuickMessageConfig, QuickMessageGroup, QuickMessageKind,
};

fn config(text: &str, group: QuickMessageGroup, kind: QuickMessageKind) -> QuickMessageConfig {
    QuickMessageConfig { text: text.to_string(), group, kind }
}

fn installed_skill(skill_id: u64, slug: &str, name: &str) -> InstalledSkill {
    InstalledSkill {
        skill_id,
        slug: slug.to_string(),
        name: name.to_string(),
        description: format!("{name} 的完整说明"),
        installed_version: "1.2.3".to_string(),
        current_version: "1.2.3".to_string(),
        format: "skill".to_string(),
        upgrade_available: false,
    }
}

fn package_status(
    local_version: Option<&str>,
    server_version: Option<&str>,
    refresh: ShijuPackageRefreshState,
) -> ShijuPackageStatus {
    let mut status = ShijuPackageStatus::default();
    status.local_version = local_version.map(str::to_string);
    status.server_version = server_version.map(str::to_string);
    status.refresh = refresh;
    status
}

#[test]
fn quick_message_strip_groups_items_preserves_order_and_maps_indices() {
    let items = vec![
        config("通用一", QuickMessageGroup::General, QuickMessageKind::Message),
        config("Git 一", QuickMessageGroup::Git, QuickMessageKind::Message),
        config("Git 动作", QuickMessageGroup::Git, QuickMessageKind::CreateNewSession),
        config("通用二", QuickMessageGroup::General, QuickMessageKind::Message),
    ];

    let (general, git) = grouped_indices(&items);
    assert_eq!(general, vec![0, 3]);
    assert_eq!(git, vec![1, 2]);

    let Message::Chat(message::ChatMessage::QuickMessageSelected(3)) = select_message(3) else {
        panic!("unexpected quick message selection");
    };
    let Message::Chat(message::ChatMessage::QuickMessageGroupChanged(Some(QuickMessageGroup::Git))) =
        group_state_message(Some(QuickMessageGroup::Git))
    else {
        panic!("unexpected quick message group change");
    };
}

#[test]
fn quick_message_dropdown_icon_uses_a_centered_square_slot() {
    let icon = dropdown_icon();
    let size = icon.as_widget().size();

    assert_eq!(size.width, iced::Length::Fixed(14.0));
    assert_eq!(size.height, iced::Length::Fixed(14.0));
}

#[test]
fn quick_message_menu_scrolls_only_after_visible_item_limit() {
    assert!(!menu_requires_scrolling(QUICK_MESSAGE_MENU_MAX_VISIBLE_ITEMS));
    assert!(menu_requires_scrolling(QUICK_MESSAGE_MENU_MAX_VISIBLE_ITEMS + 1));
    assert_eq!(menu_height(QUICK_MESSAGE_MENU_MAX_VISIBLE_ITEMS), iced::Length::Shrink);
    assert_eq!(
        menu_height(QUICK_MESSAGE_MENU_MAX_VISIBLE_ITEMS + 1),
        iced::Length::Fixed(QUICK_MESSAGE_MENU_MAX_HEIGHT)
    );
}

#[test]
fn quick_message_group_click_toggles_and_switches_open_group() {
    assert_eq!(toggled_group(None, QuickMessageGroup::General), Some(QuickMessageGroup::General));
    assert_eq!(toggled_group(Some(QuickMessageGroup::General), QuickMessageGroup::General), None);
    assert_eq!(
        toggled_group(Some(QuickMessageGroup::General), QuickMessageGroup::Git),
        Some(QuickMessageGroup::Git)
    );
}

#[test]
fn quick_message_strip_keeps_shiju_visible_and_renders_defaults_in_general_group() {
    let mut app = App::new().0;
    app.workspace_mode = WorkspaceMode::Code;
    app.quick_messages.clear();
    let empty = view(&app).expect("shiju trigger should keep the input strip visible");
    assert_eq!(Tree::new(empty.as_widget()).children.len(), 1);
    drop(empty);

    app.quick_messages = AppSystemSettingsConfig::default().quick_messages;
    assert_eq!(app.quick_messages[3].kind, QuickMessageKind::SummarizeAndCreateNewSession);
    let element = view(&app).expect("default quick messages should render");
    let tree = Tree::new(element.as_widget());
    assert_eq!(tree.children.len(), 3);
    assert_eq!(
        grouped_indices(&app.quick_messages),
        ((0..27).collect::<Vec<_>>(), (27..33).collect::<Vec<_>>())
    );
}

#[test]
fn quick_message_strip_renders_both_groups_with_original_button_indices() {
    let mut app = App::new().0;
    app.workspace_mode = WorkspaceMode::Code;
    app.quick_messages = vec![
        config("Git", QuickMessageGroup::Git, QuickMessageKind::Message),
        config("通用", QuickMessageGroup::General, QuickMessageKind::Message),
    ];

    let element = view(&app).expect("mixed quick messages should render");
    let tree = Tree::new(element.as_widget());
    assert_eq!(tree.children.len(), 3);
    drop(element);

    app.quick_message_open_group = Some(QuickMessageGroup::Git);
    assert!(view(&app).is_some());
}

#[test]
fn work_mode_space_picker_routes_selection_and_renders_without_quick_messages() {
    let mut app = App::new().0;
    app.workspace_mode = WorkspaceMode::Work;
    app.quick_messages.clear();
    app.recent_projects = vec!["/workspace/alpha".into(), "/workspace/beta".into()];
    app.project_path = Some("/workspace/alpha".into());

    assert!(view(&app).is_some());
    let Message::Batch(messages) = workspace_select_message("/workspace/beta".to_string()) else {
        panic!("workspace selection should close the picker and switch the project");
    };
    assert!(matches!(
        messages.as_slice(),
        [
            Message::View(message::ViewMessage::WorkSpacePickerClosed),
            Message::Project(message::ProjectMessage::SwitchWorkSpacePressed(path))
        ] if path == "/workspace/beta"
    ));

    app.workspace_mode = WorkspaceMode::Code;
    assert!(view(&app).is_some());
}

#[test]
fn work_mode_space_picker_add_project_closes_and_opens_folder() {
    let mut app = App::new().0;
    app.workspace_mode = WorkspaceMode::Work;
    app.work_space_picker_open = true;
    app.quick_messages.clear();
    app.recent_projects.clear();

    assert!(view(&app).is_some());
    let Message::Batch(messages) = workspace_add_message() else {
        panic!("add project should close the picker and open a folder dialog");
    };
    assert!(matches!(
        messages.as_slice(),
        [
            Message::View(message::ViewMessage::WorkSpacePickerClosed),
            Message::Project(message::ProjectMessage::OpenFolderPressed)
        ]
    ));
}

#[test]
fn input_strip_places_shiju_before_general_actions() {
    let mut app = App::new().0;
    app.workspace_mode = WorkspaceMode::Code;
    app.quick_messages = vec![
        config("通用", QuickMessageGroup::General, QuickMessageKind::Message),
        config("Git", QuickMessageGroup::Git, QuickMessageKind::Message),
    ];

    assert_eq!(
        input_strip_control_order(&app),
        vec![
            InputStripControl::Shiju,
            InputStripControl::GeneralActions,
            InputStripControl::GitActions,
        ]
    );
    let element = view(&app).expect("shiju and quick-message controls should render");
    assert_eq!(Tree::new(element.as_widget()).children.len(), 3);
    drop(element);

    app.workspace_mode = WorkspaceMode::Work;
    assert_eq!(
        input_strip_control_order(&app),
        vec![
            InputStripControl::Workspace,
            InputStripControl::Shiju,
            InputStripControl::GeneralActions,
            InputStripControl::GitActions,
        ]
    );
}

#[test]
fn shiju_trigger_count_reflects_installed_load_state() {
    assert_eq!(shiju_trigger_label(&LoadState::Idle), "识矩（…）");
    assert_eq!(shiju_trigger_label(&LoadState::Loading { generation: 3 }), "识矩（…）");
    assert_eq!(shiju_trigger_label(&LoadState::NeedsLogin), "识矩（…）");
    assert_eq!(shiju_trigger_label(&LoadState::Error("网关不可用".into())), "识矩（…）");
    assert_eq!(
        shiju_trigger_label(&LoadState::Loaded(vec![
            installed_skill(7, "first", "功能一"),
            installed_skill(8, "second", "功能二"),
        ])),
        "识矩（2）"
    );

    assert_eq!(
        shiju_menu_status(&LoadState::<Vec<InstalledSkill>>::Idle),
        ShijuMenuStatus::Loading
    );
    assert_eq!(
        shiju_menu_status(&LoadState::Loading { generation: 3 }),
        ShijuMenuStatus::Loading
    );
    assert_eq!(
        shiju_menu_status(&LoadState::NeedsLogin),
        ShijuMenuStatus::NeedsLogin
    );
    assert_eq!(
        shiju_menu_status(&LoadState::Loaded(Vec::<InstalledSkill>::new())),
        ShijuMenuStatus::Empty
    );
    assert_eq!(
        shiju_menu_status(&LoadState::Error("网关不可用".into())),
        ShijuMenuStatus::Error("网关不可用")
    );
    assert_eq!(
        shiju_menu_status_text(ShijuMenuStatus::Loading),
        "正在加载识矩技能…"
    );
    assert_eq!(
        shiju_menu_status_text(ShijuMenuStatus::NeedsLogin),
        "请先登录识矩账户"
    );
    assert_eq!(shiju_menu_status_text(ShijuMenuStatus::Empty), "暂无已安装识矩技能");
    assert_eq!(
        shiju_menu_status_text(ShijuMenuStatus::Error("网关不可用")),
        "加载失败：网关不可用"
    );
}

#[test]
fn shiju_trigger_tooltip_reports_local_server_and_upgrade_state() {
    let uninstalled =
        package_status(None, Some("2.0.0"), ShijuPackageRefreshState::Fresh);
    assert_eq!(
        shiju_version_hint(&uninstalled),
        ShijuVersionHint {
            local_version: "未安装".to_string(),
            server_version: "2.0.0".to_string(),
            detail: "本地未安装，建议前往安装".to_string(),
            upgrade_enabled: true,
        }
    );

    let checking = package_status(
        Some("1.0.0"),
        None,
        ShijuPackageRefreshState::Refreshing { generation: 7 },
    );
    assert_eq!(
        shiju_version_hint(&checking),
        ShijuVersionHint {
            local_version: "1.0.0".to_string(),
            server_version: "检测中".to_string(),
            detail: "正在检测服务端版本…".to_string(),
            upgrade_enabled: false,
        }
    );

    let current =
        package_status(Some("1.2.3"), Some("1.2.3"), ShijuPackageRefreshState::Fresh);
    assert_eq!(
        shiju_version_hint(&current),
        ShijuVersionHint {
            local_version: "1.2.3".to_string(),
            server_version: "1.2.3".to_string(),
            detail: "当前版本已是最新".to_string(),
            upgrade_enabled: false,
        }
    );

    let upgrade =
        package_status(Some("1.2.3"), Some("2.0.0"), ShijuPackageRefreshState::Fresh);
    assert_eq!(
        shiju_version_hint(&upgrade),
        ShijuVersionHint {
            local_version: "1.2.3".to_string(),
            server_version: "2.0.0".to_string(),
            detail: "版本不一致，建议前往处理".to_string(),
            upgrade_enabled: true,
        }
    );

    let stale =
        package_status(Some("1.2.3"), Some("2.0.0"), ShijuPackageRefreshState::Failed);
    assert_eq!(
        shiju_version_hint(&stale),
        ShijuVersionHint {
            local_version: "1.2.3".to_string(),
            server_version: "2.0.0（上次检测）".to_string(),
            detail: "刷新失败，建议按上次结果前往处理".to_string(),
            upgrade_enabled: true,
        }
    );

    let tooltip = shiju_trigger_tooltip(&upgrade);
    assert_eq!(Tree::new(tooltip.as_widget()).children.len(), 4);

    let mut app = App::new().0;
    app.expert_hub.package_status = upgrade;
    let menu = shiju_menu(&app);
    assert_eq!(
        Tree::new(menu.as_widget()).children.len(),
        2,
        "trigger Tooltip and menu overlay should both be present"
    );

    let Message::Batch(messages) = shiju_upgrade_message() else {
        panic!("upgrade action should close the menu before opening the install guide");
    };
    assert!(matches!(
        messages.as_slice(),
        [
            Message::Chat(message::ChatMessage::ToggleShijuSkillMenu),
            Message::SkillHub(message::SkillHubMessage::OpenInstallGuide),
        ]
    ));
}

#[test]
fn shiju_menu_keeps_refresh_action_fixed_and_maps_loading_state() {
    let idle = shiju_refresh_action(&LoadState::<Vec<InstalledSkill>>::Idle);
    let loading =
        shiju_refresh_action(&LoadState::<Vec<InstalledSkill>>::Loading { generation: 3 });
    let needs_login = shiju_refresh_action(&LoadState::<Vec<InstalledSkill>>::NeedsLogin);
    let empty = shiju_refresh_action(&LoadState::Loaded(Vec::<InstalledSkill>::new()));
    let error =
        shiju_refresh_action(&LoadState::<Vec<InstalledSkill>>::Error("网关不可用".into()));
    let loaded = shiju_refresh_action(&LoadState::Loaded(vec![installed_skill(
        7,
        "document-review",
        "文档审查",
    )]));

    assert_eq!(idle, ShijuRefreshAction { label: "刷新功能", enabled: true });
    assert_eq!(needs_login, idle);
    assert_eq!(empty, idle);
    assert_eq!(error, idle);
    assert_eq!(loaded, idle);
    assert_eq!(loading, ShijuRefreshAction { label: "刷新中…", enabled: false });
    assert!(matches!(
        shiju_refresh_message(),
        Message::SkillHub(message::SkillHubMessage::RefreshInstalledSkills)
    ));

    let mut app = App::new().0;
    app.expert_hub.installed = LoadState::Loaded(vec![installed_skill(
        7,
        "document-review",
        "文档审查",
    )]);
    let menu = shiju_menu_view(&app);
    let tree = Tree::new(menu.as_widget());
    assert_eq!(
        tree.children.len(),
        3,
        "fixed upgrade and refresh actions should precede the scrollable list"
    );
}

#[test]
fn shiju_menu_rows_map_ids_and_selected_state() {
    let mut app = App::new().0;
    app.expert_hub.installed = LoadState::Loaded(vec![
        installed_skill(17, "document-review", "文档审查"),
        installed_skill(29, "data-brief", "数据简报"),
    ]);
    app.current_session_runtime_mut().tool_selector.toggle_shiju_skill(
        crate::app::state::ShijuSkillSelection::new(
            17,
            "document-review".to_string(),
            "文档审查".to_string(),
        ),
    );
    app.current_session_runtime_mut().tool_selector.toggle_shiju_skill(
        crate::app::state::ShijuSkillSelection::new(
            29,
            "data-brief".to_string(),
            "数据简报".to_string(),
        ),
    );

    let rows = shiju_menu_rows(&app);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].skill_id, 17);
    assert_eq!(rows[0].name, "文档审查");
    assert!(rows[0].selected);
    assert_eq!(
        rows[0].tooltip,
        "文档审查\n标识：document-review\n版本：1.2.3\n\n文档审查 的完整说明"
    );
    assert_eq!(rows[1].skill_id, 29);
    assert!(rows[1].selected);

    let Message::Chat(message::ChatMessage::SelectShijuSkill(29)) =
        shiju_select_message(rows[1].skill_id)
    else {
        panic!("shiju row should map its stable skill id");
    };
}
