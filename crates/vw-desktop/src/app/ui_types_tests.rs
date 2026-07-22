#[test]
fn settings_tab_display_labels_are_stable() {
    assert_eq!(super::SettingsTab::Settings.to_string(), "设置");
    assert_eq!(super::SettingsTab::SettingsJson.to_string(), "JSON配置");
    assert_eq!(super::SettingsTab::Files.to_string(), "文件管理器");
    assert_eq!(super::SettingsTab::Projects.to_string(), "项目管理器");
    assert_eq!(super::SettingsTab::Sessions.to_string(), "历史会话");
}

#[test]
fn settings_tab_all_keeps_primary_tabs_only() {
    assert_eq!(
        super::SettingsTab::all(),
        [super::SettingsTab::Settings, super::SettingsTab::Files, super::SettingsTab::Projects]
    );
}

#[test]
fn screen_variants_exclude_removed_apps() {
    fn screen_name(screen: super::Screen) -> &'static str {
        match screen {
            super::Screen::Home => "Home",
            super::Screen::Project => "Project",
            super::Screen::Preview => "Preview",
            super::Screen::Apps => "Apps",
            super::Screen::Usage => "Usage",
            super::Screen::Knowledge => "Knowledge",
            super::Screen::WorkflowTool => "WorkflowTool",
            super::Screen::WorkflowManager => "WorkflowManager",
            super::Screen::TaskBoard => "TaskBoard",
        }
    }

    let screens = [
        super::Screen::Home,
        super::Screen::Project,
        super::Screen::Preview,
        super::Screen::Apps,
        super::Screen::Usage,
        super::Screen::Knowledge,
        super::Screen::WorkflowTool,
        super::Screen::WorkflowManager,
        super::Screen::TaskBoard,
    ];

    assert!(!screens.iter().map(|screen| screen_name(*screen)).any(|name| name == "Design"));
    assert!(!include_str!("message/mod.rs").contains(concat!("Design", "Message")));
}
