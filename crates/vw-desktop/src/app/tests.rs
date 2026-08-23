//! 桌面应用顶层测试，验证测试模块接线与项目配置目录命名空间。
//! 配置路径必须指向当前应用范围，避免与历史或其他产品目录混用。

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("tests"));
}

#[test]
fn project_dirs_points_at_current_application_namespace() {
    let dirs = super::project_dirs().expect("project dirs should resolve on desktop platforms");

    let config_dir = dirs.config_dir().to_string_lossy();
    assert!(config_dir.ends_with("vibe-window-dev") || config_dir.ends_with("vibe-window"));
}
