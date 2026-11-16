//! Installation directory projection uses the same domain entry as the CLI.
use crate::cli::install_dir_from_config;
use agent_config::runtime::Config;
use std::path::Path;

#[test]
fn plugin_install_dir_uses_config_parent() {
    let mut config = Config::default();
    config.config_path = "/tmp/vibewindow-config/vibewindow.json".into();

    assert_eq!(install_dir_from_config(&config), Path::new("/tmp/vibewindow-config/plugins"));
}
