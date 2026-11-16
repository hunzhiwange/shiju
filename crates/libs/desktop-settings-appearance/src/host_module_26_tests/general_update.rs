use crate::general_update::*;
use desktop_settings_shell::GeneralSettingsTab;
#[test]
fn tab_selection_updates_general_settings_content() {
    let mut tab = GeneralSettingsTab::Interface;
    assert!(!select_tab(&mut tab, &mut None, GeneralSettingsTab::Editor));
    assert_eq!(tab, GeneralSettingsTab::Editor);
}
#[test]
fn theme_tab_selection_clears_error_and_refreshes_gallery() {
    let mut tab = GeneralSettingsTab::Interface;
    let mut error = Some("stale".to_owned());
    assert!(select_tab(&mut tab, &mut error, GeneralSettingsTab::Themes));
    assert_eq!(tab, GeneralSettingsTab::Themes);
    assert_eq!(error, None);
}
#[test]
fn timezone_selection_rejects_unknown_value() {
    let mut zone = "UTC".to_owned();
    assert!(!select_timezone(&mut zone, "Mars/Olympus_Mons"));
    assert_eq!(zone, "UTC");
    assert!(select_timezone(&mut zone, "Asia/Shanghai"));
    assert_eq!(zone, "Asia/Shanghai");
}
