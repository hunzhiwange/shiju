use super::*;

// 验证 `footer_collapses_for_narrow_widths` 场景的触发条件与预期结果。
#[test]
fn footer_collapses_for_narrow_widths() {
    let props = FooterProps {
        mode: FooterMode::ComposerHasDraft,
        data: V3FooterData {
            model: Some("model".into()),
            permission: Some("workspace".into()),
            directory: Some("/tmp/project".into()),
            context_window_percent: Some(42),
            context_window_used_tokens: Some(12_345),
            is_task_running: true,
        },
        queued_count: 2,
        attachment_count: 1,
    };
    assert!(footer_line(&props, 40).width() <= 40);
    assert!(footer_line(&props, 80).width() <= 80);
    assert!(footer_line(&props, 120).width() <= 120);
    assert_eq!(footer_height(&props, 10), 0);
}
