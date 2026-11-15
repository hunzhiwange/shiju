use super::{default_workspace_identity_root, workspace_identity_hint, format_file_size};
#[test]
fn default_workspace_identity_root_uses_agent_key() {
    let config_root = format!("~/{}", config_runtime_paths::HOME_CONFIG_DIR_NAME);

    assert_eq!(default_workspace_identity_root("main"), format!("{config_root}/workspace"));
    assert_eq!(default_workspace_identity_root("coder"), format!("{config_root}/workspace-coder"));
}

#[test]
fn workspace_identity_hint_prefers_custom_root() {
    assert_eq!(
        workspace_identity_hint(
            config_ui::DesktopLocale::SimplifiedChinese,
            Some("profiles/reviewer"),
            "reviewer",
            "AGENTS.md"
        ),
        "编辑身份文件 `AGENTS.md`，保存会直接写回 `profiles/reviewer/AGENTS.md`。"
    );
}

#[test]
fn workspace_identity_hint_uses_default_root_for_main_and_workers() {
    assert_eq!(
        workspace_identity_hint(
            config_ui::DesktopLocale::SimplifiedChinese,
            None,
            "main",
            "AGENTS.md"
        ),
        format!(
            "编辑身份文件 `AGENTS.md`，保存会直接写回 `~/{}/workspace/AGENTS.md`。",
            config_runtime_paths::HOME_CONFIG_DIR_NAME
        )
    );
    assert_eq!(
        workspace_identity_hint(
            config_ui::DesktopLocale::SimplifiedChinese,
            None,
            "reviewer",
            "AGENTS.md"
        ),
        format!(
            "编辑身份文件 `AGENTS.md`，保存会直接写回 `~/{}/workspace-reviewer/AGENTS.md`。",
            config_runtime_paths::HOME_CONFIG_DIR_NAME
        )
    );
}

#[test]
fn format_file_size_uses_human_readable_units() {
    assert_eq!(format_file_size(None), None);
    assert_eq!(format_file_size(Some(0)), Some("0 B".to_string()));
    assert_eq!(format_file_size(Some(512)), Some("512 B".to_string()));
    assert_eq!(format_file_size(Some(1023)), Some("1023 B".to_string()));
    assert_eq!(format_file_size(Some(1024)), Some("1.0 KB".to_string()));
    assert_eq!(format_file_size(Some(2048)), Some("2.0 KB".to_string()));
    assert_eq!(format_file_size(Some(1024 * 1024)), Some("1.0 MB".to_string()));
    assert_eq!(format_file_size(Some(2 * 1024 * 1024 + 512 * 1024)), Some("2.5 MB".to_string()));
}

use super::{workspace_identity_meta_line, identity_file_is_enabled};
#[test]
fn workspace_identity_meta_line_combines_size_and_modified_time() {
    assert_eq!(
        workspace_identity_meta_line(
            config_ui::DesktopLocale::SimplifiedChinese,
            Some(1536),
            Some((0, 0))
        ),
        Some("1.5 KB · 刚刚".to_string())
    );
}

#[test]
fn workspace_identity_meta_line_handles_single_and_missing_values() {
    assert_eq!(
        workspace_identity_meta_line(
            config_ui::DesktopLocale::SimplifiedChinese,
            Some(512),
            None
        ),
        Some("512 B".to_string())
    );
    assert_eq!(
        workspace_identity_meta_line(
            config_ui::DesktopLocale::SimplifiedChinese,
            None,
            Some((0, 0))
        ),
        Some("刚刚".to_string())
    );
    assert_eq!(
        workspace_identity_meta_line(
            config_ui::DesktopLocale::SimplifiedChinese,
            None,
            None
        ),
        None
    );
}

#[test]
fn identity_file_is_enabled_defaults_to_false_and_reflects_whitelist() {
    let mut entry = crate::state::DelegateAgentSettingsEntry::from_config("main", None);
    assert!(!identity_file_is_enabled(&entry, "AGENTS.md"));

    entry.enabled_identity_files.push("AGENTS.md".to_string());

    assert!(identity_file_is_enabled(&entry, "AGENTS.md"));
    assert!(!identity_file_is_enabled(&entry, "SOUL.md"));
}
