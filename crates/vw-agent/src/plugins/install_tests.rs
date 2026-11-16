//! 配置目录投影仍由宿主组合层持有。
use super::install::install_dir_from_config;
use crate::config::Config;
use std::path::Path;

#[test]
fn plugin_install_dir_uses_config_parent() {
    let mut config = Config::default();
    config.config_path = "/tmp/vibewindow-config/vibewindow.json".into();

    assert_eq!(install_dir_from_config(&config), Path::new("/tmp/vibewindow-config/plugins"));
}
