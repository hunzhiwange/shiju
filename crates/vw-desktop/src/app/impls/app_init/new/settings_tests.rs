//! 验证新建应用时各类设置状态的组装和默认值。
//!
//! 设置构造同时检查默认配置和显式覆盖，防止新增字段未进入应用初始状态。

#[test]
fn settings_tests_module_is_wired() {
    assert!(module_path!().ends_with("settings_tests"));
}

#[test]
fn build_multimodal_settings_loads_ocr_config() {
    let mut config = vw_config_types::config::Config::default();
    config.multimodal.max_images = 8;
    config.multimodal.max_image_size_mb = 10;
    config.multimodal.allow_remote_fetch = true;
    config.multimodal.ocr.enabled = true;
    config.multimodal.ocr.model_supports_images = false;
    config.multimodal.ocr.model = Some(" vision-provider/vision-model ".to_string());

    let settings = super::build_multimodal_settings(&config);

    assert_eq!(settings.max_images, 8);
    assert_eq!(settings.max_image_size_mb, 10);
    assert!(settings.allow_remote_fetch);
    assert!(settings.ocr_enabled);
    assert!(!settings.model_supports_images);
    assert_eq!(settings.ocr_model, "vision-provider/vision-model");
    assert!(settings.save_error.is_none());
}

#[test]
fn legacy_task_board_switches_are_ignored_after_capability_cutover() {
    let cfg = serde_json::json!({
        "task_board_auto_promote_pool_tasks": true,
        "task_board_code_review_enabled": true,
        "task_board_task_split_enabled": true,
        "auto_execute": true,
        "auto_promote_pool_tasks": true,
        "code_review_enabled": true,
        "task_split_enabled": true,
        "max_concurrent": 3,
        "default_priority": 999,
        "auto_promote_delay_seconds": 30
    });

    let settings = super::build_task_board_settings(&cfg);
    let encoded = serde_json::to_value(&settings).expect("settings should serialize");
    for key in
        ["auto_execute", "auto_promote_pool_tasks", "code_review_enabled", "task_split_enabled"]
    {
        assert!(encoded.get(key).is_none(), "legacy setting {key} should not be persisted");
    }

    let legacy: crate::app::task::TaskBoardSettings =
        serde_json::from_value(cfg).expect("legacy task board settings should decode");
    let legacy_encoded = serde_json::to_value(&legacy).expect("legacy settings should serialize");
    for key in
        ["auto_execute", "auto_promote_pool_tasks", "code_review_enabled", "task_split_enabled"]
    {
        assert!(
            legacy_encoded.get(key).is_none(),
            "legacy decoded setting {key} should be ignored"
        );
    }

    let control_source = include_str!("../../../views/task_board/control.rs");
    assert!(!control_source.contains("自动执行任务池 ·"));
    assert!(!control_source.contains("代码审核 ·"));
    assert!(!control_source.contains("任务拆分 ·"));
    assert!(!control_source.contains("ToggleAutoPromotePoolTasks"));
    assert!(!control_source.contains("ToggleCodeReview"));
    assert!(!control_source.contains("ToggleTaskSplit"));

    let modal_source = include_str!("../../../views/task_board/modals.rs");
    assert!(!modal_source.contains("task_split_enabled"));
    assert!(!modal_source.contains("ToggleTaskSplit"));
    assert!(!modal_source.contains("field_row(\"任务拆分\""));

    let project_refresh_source = include_str!("../../app_view_modals/project_edit/refresh_tab.rs");
    assert!(!project_refresh_source.contains("自动拉起任务池"));
    assert!(!project_refresh_source.contains("执行代码审查"));
    assert!(!project_refresh_source.contains("ProjectEditAutoPromotePoolTasksToggled"));
    assert!(!project_refresh_source.contains("ProjectEditCodeReviewToggled"));
}
