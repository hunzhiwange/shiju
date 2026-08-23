//! 验证 launchd 标识、目标解析及 plist 参数和环境渲染。

use super::macos::{
    SERVICE_LABEL, build_launchd_plist, config_dir_arg, launchd_gui_service_target_for_uid,
    macos_service_file,
};
use crate::app::agent::config::Config;
use std::path::Path;

#[test]
fn service_label_matches_launchd_identifier() {
    assert_eq!(SERVICE_LABEL, "com.vibewindow.daemon");
}

#[test]
fn config_dir_arg_returns_config_parent_without_file_name() {
    let mut config = Config::default();
    config.config_path = "/Users/me/Library/Application Support/VibeWindow/vibewindow.json".into();

    let dir = config_dir_arg(&config).unwrap();

    assert_eq!(dir, "/Users/me/Library/Application Support/VibeWindow");
    assert!(!dir.contains("vibewindow.json"));
}

#[test]
fn config_dir_arg_returns_none_when_config_path_has_no_parent() {
    let mut config = Config::default();
    config.config_path = "/".into();

    assert_eq!(config_dir_arg(&config), None);
}

#[test]
fn macos_service_file_uses_launch_agents_label() {
    let file = macos_service_file().unwrap();

    assert!(file.ends_with("Library/LaunchAgents/com.vibewindow.daemon.plist"));
}

#[test]
fn launchd_plist_includes_escaped_arguments_log_paths_and_environment() {
    let env_block = "\n  <key>EnvironmentVariables</key>\n  <dict>\n    <key>OPENAI_API_KEY</key>\n    <string>test-value</string>\n  </dict>";

    let plist = build_launchd_plist(
        Path::new("/Applications/Vibe & Window/<daemon>\"'"),
        Some("/Users/me/A&B/<config>\"'"),
        Path::new("/tmp/a&b/stdout<1>.log"),
        Path::new("/tmp/a&b/stderr\"'.log"),
        env_block,
    );

    assert!(plist.contains("<string>com.vibewindow.daemon</string>"));
    assert!(
        plist.contains(
            "<string>/Applications/Vibe &amp; Window/&lt;daemon&gt;&quot;&apos;</string>"
        )
    );
    assert!(plist.contains("<string>--config-dir</string>"));
    assert!(plist.contains("<string>/Users/me/A&amp;B/&lt;config&gt;&quot;&apos;</string>"));
    assert!(plist.contains("<string>/tmp/a&amp;b/stdout&lt;1&gt;.log</string>"));
    assert!(plist.contains("<string>/tmp/a&amp;b/stderr&quot;&apos;.log</string>"));
    assert!(plist.contains(env_block));

    let exe_position = plist.find("/Applications/Vibe").unwrap();
    let config_flag_position = plist.find("--config-dir").unwrap();
    let config_position = plist.find("/Users/me").unwrap();
    let daemon_position = plist.rfind("<string>daemon</string>").unwrap();
    assert!(exe_position < config_flag_position);
    assert!(config_flag_position < config_position);
    assert!(config_position < daemon_position);
}

#[test]
fn launchd_plist_omits_config_argument_and_environment_when_absent() {
    let plist = build_launchd_plist(
        Path::new("/usr/local/bin/vibewindow"),
        None,
        Path::new("logs/daemon.stdout.log"),
        Path::new("logs/daemon.stderr.log"),
        "",
    );

    assert!(!plist.contains("--config-dir"));
    assert!(!plist.contains("EnvironmentVariables"));
    assert!(plist.contains("<string>/usr/local/bin/vibewindow</string>"));
    assert!(plist.contains("<string>daemon</string>"));
    assert!(plist.contains("<string>logs/daemon.stdout.log</string>"));
    assert!(plist.contains("<string>logs/daemon.stderr.log</string>"));
}

#[test]
fn launchd_plist_preserves_explicit_empty_config_directory_argument() {
    let plist = build_launchd_plist(
        Path::new("/usr/local/bin/vibewindow"),
        Some(""),
        Path::new("stdout.log"),
        Path::new("stderr.log"),
        "",
    );

    assert!(plist.contains("<string>--config-dir</string>\n    <string></string>"));
}

#[test]
fn launchd_gui_service_target_trims_uid_and_preserves_ascii_digits() {
    assert_eq!(
        launchd_gui_service_target_for_uid(" 501\n"),
        Some("gui/501/com.vibewindow.daemon".to_string())
    );
    assert_eq!(
        launchd_gui_service_target_for_uid("00042"),
        Some("gui/00042/com.vibewindow.daemon".to_string())
    );
    assert_eq!(
        launchd_gui_service_target_for_uid("0"),
        Some("gui/0/com.vibewindow.daemon".to_string())
    );
}

#[test]
fn launchd_gui_service_target_rejects_empty_signed_mixed_and_non_ascii_uids() {
    for uid in ["", "  ", "abc", "50a", "-1", "+501", "501 2", "５０１"] {
        assert_eq!(launchd_gui_service_target_for_uid(uid), None, "uid: {uid}");
    }
}
